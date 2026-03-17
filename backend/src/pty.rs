//! PTY abstraction for the Rust backend.
//!
//! This layer owns the platform-specific process and PTY resources so the
//! WebSocket route can focus on protocol behavior:
//! - write client bytes into the PTY stdin
//! - resize the PTY
//! - stream PTY stdout/stderr back to the session
//! - detect process exit and surface the exit reason
//!
//! The helpers below also encode the compatibility-sensitive shell launch
//! contract that the backend relies on, so it can be tested without spawning a
//! real PTY.

use std::{
    collections::BTreeMap,
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow, bail};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::{
    sync::{mpsc, watch},
    task,
};

use crate::{
    contract::{Dimensions, PtyExitReason},
    error::AppError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySpec {
    pub dimensions: Dimensions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellLaunchSpec {
    executable: PathBuf,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
}

pub struct PtyHandle {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    process_id: Option<i32>,
    output_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    exit_rx: watch::Receiver<Option<PtyExitReason>>,
}

impl PtyHandle {
    /// Write raw bytes to the PTY stdin stream.
    ///
    /// # Errors
    ///
    /// Returns an error when the writer mutex is poisoned, the blocking task
    /// fails, or the PTY input cannot be written and flushed.
    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let writer = Arc::clone(&self.writer);
        let owned = bytes.to_vec();
        task::spawn_blocking(move || -> Result<()> {
            let mut writer = writer
                .lock()
                .map_err(|_| anyhow!("writer mutex poisoned"))?;
            writer
                .write_all(&owned)
                .context("failed to write PTY input")?;
            writer.flush().context("failed to flush PTY input")?;
            drop(writer);
            Ok(())
        })
        .await
        .map_err(|error| anyhow!("PTY write task failed: {error}"))??;

        Ok(())
    }

    /// Resize the PTY to the requested dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error when the master mutex is poisoned, the blocking task
    /// fails, or the PTY resize operation itself fails.
    pub async fn resize(&mut self, dimensions: Dimensions) -> Result<()> {
        let master = Arc::clone(&self.master);
        task::spawn_blocking(move || -> Result<()> {
            let master = master
                .lock()
                .map_err(|_| anyhow!("master mutex poisoned"))?;
            master
                .resize(to_pty_size(dimensions))
                .context("failed to resize PTY")?;
            drop(master);
            Ok(())
        })
        .await
        .map_err(|error| anyhow!("PTY resize task failed: {error}"))??;

        Ok(())
    }

    /// Send `SIGTERM` to the PTY child process when one is still running.
    ///
    /// # Errors
    ///
    /// Returns an error when the blocking task fails or the process cannot be
    /// terminated for a reason other than already having exited.
    pub async fn kill(&mut self) -> Result<()> {
        let Some(process_id) = self.process_id else {
            return Ok(());
        };

        task::spawn_blocking(move || -> Result<()> {
            let result = unsafe { libc::kill(process_id, libc::SIGTERM) };
            if result == 0 {
                Ok(())
            } else {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::ESRCH) {
                    Ok(())
                } else {
                    Err(anyhow!("failed to terminate PTY process: {error}"))
                }
            }
        })
        .await
        .map_err(|error| anyhow!("PTY kill task failed: {error}"))??;

        Ok(())
    }

    pub async fn next_output(&mut self) -> Option<Vec<u8>> {
        self.output_rx.recv().await
    }

    /// Clone the watch receiver used to observe PTY exit reasons.
    #[must_use]
    pub fn exit_receiver(&self) -> watch::Receiver<Option<PtyExitReason>> {
        self.exit_rx.clone()
    }
}

/// Spawn a shell inside a PTY using the backend's compatibility launch
/// settings.
///
/// # Errors
///
/// Returns an error when PTY resources cannot be created, the shell launch
/// contract cannot be resolved, or background worker setup fails.
pub async fn spawn_pty(spec: PtySpec) -> Result<PtyHandle> {
    task::spawn_blocking(move || spawn_pty_blocking(&spec))
        .await
        .map_err(|error| anyhow!("PTY spawn task failed: {error}"))?
}

fn spawn_pty_blocking(spec: &PtySpec) -> Result<PtyHandle> {
    let system = native_pty_system();
    let pair = system
        .openpty(to_pty_size(spec.dimensions))
        .context("failed to open PTY pair")?;
    let launch = resolve_shell_launch_spec()?;

    let mut command = CommandBuilder::new(
        launch
            .executable
            .to_str()
            .ok_or_else(|| anyhow!("shell path must be valid UTF-8"))?,
    );
    command.cwd(
        launch
            .cwd
            .to_str()
            .ok_or_else(|| anyhow!("shell cwd must be valid UTF-8"))?,
    );

    for (key, value) in &launch.env {
        command.env(key, value);
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .context("failed to spawn shell inside PTY")?;
    drop(pair.slave);

    let process_id = child.process_id().and_then(|pid| i32::try_from(pid).ok());
    let writer = pair
        .master
        .take_writer()
        .context("failed to open PTY writer")?;
    let reader = pair
        .master
        .try_clone_reader()
        .context("failed to clone PTY reader")?;

    let master = Arc::new(Mutex::new(pair.master));
    let writer = Arc::new(Mutex::new(writer));
    let (output_tx, output_rx) = mpsc::unbounded_channel();
    let (exit_tx, exit_rx) = watch::channel(None);

    std::thread::Builder::new()
        .name("commut-pty-reader".to_owned())
        .spawn(move || pump_pty_output(reader, &output_tx))
        .context("failed to spawn PTY reader thread")?;

    std::thread::Builder::new()
        .name("commut-pty-wait".to_owned())
        .spawn(move || {
            let reason = child.wait().map_or(
                PtyExitReason {
                    exit_code: None,
                    signal: None,
                },
                |status| PtyExitReason {
                    exit_code: i32::try_from(status.exit_code()).ok(),
                    signal: None,
                },
            );
            let _ = exit_tx.send(Some(reason));
        })
        .context("failed to spawn PTY wait thread")?;

    Ok(PtyHandle {
        master,
        writer,
        process_id,
        output_rx,
        exit_rx,
    })
}

fn pump_pty_output(mut reader: Box<dyn Read + Send>, output_tx: &mpsc::UnboundedSender<Vec<u8>>) {
    let mut buffer = [0_u8; 8192];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if output_tx.send(buffer[..read].to_vec()).is_err() {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}

const fn to_pty_size(dimensions: Dimensions) -> PtySize {
    PtySize {
        rows: dimensions.rows,
        cols: dimensions.cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn resolve_shell_launch_spec() -> Result<ShellLaunchSpec> {
    shell_launch_spec_with(
        |name| env::var_os(name).map(PathBuf::from),
        is_executable_file,
    )
}

fn shell_launch_spec_with<GetEnv, Exists>(
    get_env: GetEnv,
    exists: Exists,
) -> Result<ShellLaunchSpec>
where
    GetEnv: Fn(&str) -> Option<PathBuf>,
    Exists: Fn(&Path) -> bool,
{
    let home = get_env("HOME")
        .ok_or_else(|| anyhow!(AppError::internal("$HOME cannot be empty").message))?;
    if home.as_os_str().is_empty() {
        bail!("{}", AppError::internal("$HOME cannot be empty").message);
    }

    let executable = resolve_shell_path_with(&home, &exists)?;
    let mut launch_env = BTreeMap::new();
    launch_env.insert(
        "PATH".to_owned(),
        format!("{}:/usr/bin", home.join(".nix-profile/bin").display()),
    );
    launch_env.insert("TERM".to_owned(), "xterm-256color".to_owned());
    launch_env.insert("COLORTERM".to_owned(), "truecolor".to_owned());
    launch_env.insert("NODE_PTY".to_owned(), "1".to_owned());

    if let Some(user) = get_env("USER").filter(|value| !value.as_os_str().is_empty()) {
        launch_env.insert("USER".to_owned(), user.to_string_lossy().into_owned());
    }
    if let Some(runtime_dir) =
        get_env("XDG_RUNTIME_DIR").filter(|value| !value.as_os_str().is_empty())
    {
        launch_env.insert(
            "XDG_RUNTIME_DIR".to_owned(),
            runtime_dir.to_string_lossy().into_owned(),
        );
    }
    if let Some(dbus) =
        get_env("DBUS_SESSION_BUS_ADDRESS").filter(|value| !value.as_os_str().is_empty())
    {
        launch_env.insert(
            "DBUS_SESSION_BUS_ADDRESS".to_owned(),
            dbus.to_string_lossy().into_owned(),
        );
    }

    Ok(ShellLaunchSpec {
        executable,
        cwd: home,
        env: launch_env,
    })
}

fn resolve_shell_path_with<Exists>(home: &Path, exists: &Exists) -> Result<PathBuf>
where
    Exists: Fn(&Path) -> bool,
{
    let shell = home.join(".nix-profile/bin/zsh");
    if exists(&shell) {
        Ok(shell)
    } else {
        Err(anyhow!(
            "{}",
            AppError::internal("required shell executable not found at $HOME/.nix-profile/bin/zsh")
                .message
        ))
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && (metadata.permissions().mode() & 0o111) != 0
    }

    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use super::{resolve_shell_path_with, shell_launch_spec_with};

    fn env_map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn shell_resolution_prefers_the_current_typescript_target_first() {
        let existing = BTreeSet::from(["/home/tester/.nix-profile/bin/zsh".to_owned()]);

        let resolved = resolve_shell_path_with(std::path::Path::new("/home/tester"), &|path| {
            existing.contains(&path.to_string_lossy().to_string())
        })
        .expect("shell should resolve");

        assert_eq!(resolved, PathBuf::from("/home/tester/.nix-profile/bin/zsh"));
    }

    #[test]
    fn shell_resolution_requires_the_typescript_compatibility_target_exactly() {
        let error = resolve_shell_path_with(std::path::Path::new("/home/tester"), &|_path| false)
            .expect_err("missing strict shell target must fail");

        assert!(
            error
                .to_string()
                .contains("required shell executable not found at $HOME/.nix-profile/bin/zsh")
        );
    }

    #[test]
    fn shell_launch_spec_matches_the_required_environment_contract() {
        let env = env_map(&[
            ("HOME", "/home/tester"),
            ("USER", "tester"),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
            ("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus"),
        ]);

        let launch = shell_launch_spec_with(
            |name| env.get(name).map(PathBuf::from),
            |path| path == Path::new("/home/tester/.nix-profile/bin/zsh"),
        )
        .expect("launch spec should build");

        assert_eq!(
            launch.executable,
            PathBuf::from("/home/tester/.nix-profile/bin/zsh")
        );
        assert_eq!(launch.cwd, PathBuf::from("/home/tester"));
        assert_eq!(
            launch.env.get("PATH"),
            Some(&"/home/tester/.nix-profile/bin:/usr/bin".to_owned())
        );
        assert_eq!(launch.env.get("TERM"), Some(&"xterm-256color".to_owned()));
        assert_eq!(launch.env.get("COLORTERM"), Some(&"truecolor".to_owned()));
        assert_eq!(launch.env.get("NODE_PTY"), Some(&"1".to_owned()));
        assert_eq!(launch.env.get("USER"), Some(&"tester".to_owned()));
        assert_eq!(
            launch.env.get("XDG_RUNTIME_DIR"),
            Some(&"/run/user/1000".to_owned())
        );
        assert_eq!(
            launch.env.get("DBUS_SESSION_BUS_ADDRESS"),
            Some(&"unix:path=/run/user/1000/bus".to_owned())
        );
    }

    #[test]
    fn shell_launch_spec_requires_home() {
        let env = env_map(&[]);

        let error = shell_launch_spec_with(|name| env.get(name).map(PathBuf::from), |_path| true)
            .expect_err("missing HOME must fail");

        assert!(error.to_string().contains("$HOME cannot be empty"));
    }
}
