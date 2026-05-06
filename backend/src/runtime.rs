//! Runtime configuration helpers for the executable entrypoint.
//!
//! The default authorized public key location is
//! `~/.config/commut/authorized.pub.pem`. The binary may still accept explicit
//! environment overrides for local development, but the default path itself is
//! compatibility-sensitive and therefore belongs in tested library code rather
//! than ad-hoc binary logic.

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};

pub const DEFAULT_AUTHORIZED_PUBLIC_KEY_RELATIVE_PATH: &str = ".config/commut/authorized.pub.pem";
pub const BUILD_FRONTEND_ARG: &str = "--build-frontend";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CliOptions {
    pub build_frontend: bool,
}

/// Parse supported CLI flags for the backend binary.
///
/// # Errors
///
/// Returns an error when an unsupported CLI flag is supplied.
pub fn parse_cli_args() -> Result<CliOptions> {
    parse_cli_args_from(env::args_os())
}

/// Resolve the default authorized public key location under `$HOME`.
///
/// # Errors
///
/// Returns an error when `$HOME` is unset or empty.
pub fn default_authorized_public_key_path() -> Result<PathBuf> {
    default_authorized_public_key_path_with(|name| env::var_os(name).map(PathBuf::from))
}

/// Load the authorized public key PEM from the configured runtime sources.
///
/// # Errors
///
/// Returns an error when no configured source can be resolved or the selected
/// file cannot be read.
pub fn load_authorized_public_key_pem() -> Result<String> {
    load_authorized_public_key_pem_with(
        |name| env::var(name).ok(),
        |name| env::var_os(name).map(PathBuf::from),
        |path| fs::read_to_string(path).map_err(anyhow::Error::from),
    )
}

/// Build the frontend bundle and return the realized output path.
///
/// # Panics
///
/// Panics if the local system clock reports a time before the Unix epoch.
///
/// # Errors
///
/// Returns an error when the working directory cannot be resolved, `nix build`
/// fails, or the resulting output path cannot be canonicalized.
pub fn build_frontend_assets() -> Result<PathBuf> {
    let cwd = env::current_dir().context("failed to resolve current working directory")?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock should be after unix epoch")
        .as_nanos();
    let out_link = env::temp_dir().join(format!("commut-client-build-{unique}"));

    build_frontend_assets_with(&cwd, &out_link, |cwd, out_link| {
        let status = Command::new("nix")
            .current_dir(cwd)
            .args(["build", ".#commut-client", "--out-link"])
            .arg(out_link)
            .status()
            .context("failed to run `nix build .#commut-client`")?;

        if !status.success() {
            return Err(anyhow!(
                "`nix build .#commut-client` exited with status {status}"
            ));
        }

        Ok(())
    })
}

#[must_use]
pub const fn should_build_frontend_for_run(
    cli: CliOptions,
    explicit_public_dir: bool,
    explicit_build_dir: bool,
) -> bool {
    cli.build_frontend || (cfg!(debug_assertions) && (!explicit_public_dir || !explicit_build_dir))
}

fn default_authorized_public_key_path_with<GetEnvPath>(get_env_path: GetEnvPath) -> Result<PathBuf>
where
    GetEnvPath: Fn(&str) -> Option<PathBuf>,
{
    let home = get_env_path("HOME")
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| anyhow!("$HOME must be set to resolve the default authorized key path"))?;
    Ok(home.join(DEFAULT_AUTHORIZED_PUBLIC_KEY_RELATIVE_PATH))
}

fn load_authorized_public_key_pem_with<GetEnvString, GetEnvPath, ReadFile>(
    get_env_string: GetEnvString,
    get_env_path: GetEnvPath,
    read_file: ReadFile,
) -> Result<String>
where
    GetEnvString: Fn(&str) -> Option<String>,
    GetEnvPath: Fn(&str) -> Option<PathBuf>,
    ReadFile: Fn(&Path) -> Result<String>,
{
    if let Some(path) = get_env_string("COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE")
        .filter(|value| !value.trim().is_empty())
    {
        return read_file(Path::new(&path)).with_context(|| {
            format!("failed to read COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE: {path}")
        });
    }

    if let Some(pem) =
        get_env_string("COMMUT_AUTHORIZED_PUBLIC_KEY_PEM").filter(|value| !value.trim().is_empty())
    {
        return Ok(pem);
    }

    let default_path = default_authorized_public_key_path_with(get_env_path)?;
    read_file(&default_path).with_context(|| {
        format!(
            "failed to read default authorized public key at {}",
            default_path.display()
        )
    })
}

fn parse_cli_args_from<I, S>(args: I) -> Result<CliOptions>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut options = CliOptions::default();

    for arg in args.into_iter().skip(1) {
        let arg = arg.into();
        if arg == BUILD_FRONTEND_ARG {
            options.build_frontend = true;
        } else {
            return Err(anyhow!(
                "unknown argument: {}",
                PathBuf::from(&arg).display()
            ));
        }
    }

    Ok(options)
}

fn build_frontend_assets_with<Run>(cwd: &Path, out_link: &Path, run: Run) -> Result<PathBuf>
where
    Run: FnOnce(&Path, &Path) -> Result<()>,
{
    run(cwd, out_link)?;
    fs::canonicalize(out_link).with_context(|| {
        format!(
            "failed to resolve built frontend output at {}",
            out_link.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_authorized_public_key_path_uses_the_specified_home_relative_location() {
        let path = default_authorized_public_key_path_with(|name| match name {
            "HOME" => Some(PathBuf::from("/home/commut")),
            _ => None,
        })
        .expect("default path should resolve");

        assert_eq!(
            path,
            PathBuf::from("/home/commut/.config/commut/authorized.pub.pem")
        );
    }

    #[test]
    fn env_file_override_takes_precedence_over_inline_pem_and_default_path() {
        let pem = load_authorized_public_key_pem_with(
            |name| match name {
                "COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE" => Some("/override/key.pem".to_owned()),
                "COMMUT_AUTHORIZED_PUBLIC_KEY_PEM" => Some("INLINE".to_owned()),
                _ => None,
            },
            |_| Some(PathBuf::from("/home/commut")),
            |path| {
                assert_eq!(path, Path::new("/override/key.pem"));
                Ok("FROM_FILE".to_owned())
            },
        )
        .expect("file override should win");

        assert_eq!(pem, "FROM_FILE");
    }

    #[test]
    fn inline_pem_is_used_when_no_file_override_is_present() {
        let pem = load_authorized_public_key_pem_with(
            |name| match name {
                "COMMUT_AUTHORIZED_PUBLIC_KEY_PEM" => Some("INLINE".to_owned()),
                _ => None,
            },
            |_| Some(PathBuf::from("/home/commut")),
            |_| bail!("default path should not be read when inline pem exists"),
        )
        .expect("inline PEM should be accepted");

        assert_eq!(pem, "INLINE");
    }

    #[test]
    fn default_home_relative_path_is_used_when_no_env_override_exists() {
        let pem = load_authorized_public_key_pem_with(
            |_| None,
            |name| match name {
                "HOME" => Some(PathBuf::from("/home/commut")),
                _ => None,
            },
            |path| {
                assert_eq!(
                    path,
                    Path::new("/home/commut/.config/commut/authorized.pub.pem")
                );
                Ok("FROM_DEFAULT".to_owned())
            },
        )
        .expect("default path should be used");

        assert_eq!(pem, "FROM_DEFAULT");
    }

    #[test]
    fn default_loader_reads_a_real_file_from_the_specified_default_location() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let home = std::env::temp_dir().join(format!("commut-runtime-test-{unique}"));
        let key_path = home.join(DEFAULT_AUTHORIZED_PUBLIC_KEY_RELATIVE_PATH);
        fs::create_dir_all(
            key_path
                .parent()
                .expect("default authorized key path must have a parent"),
        )
        .expect("temp key directory should be created");
        fs::write(&key_path, "PEM_FROM_DEFAULT_FILE").expect("temp key should be written");

        let pem = load_authorized_public_key_pem_with(
            |_| None,
            |name| match name {
                "HOME" => Some(home.clone()),
                _ => None,
            },
            |path| fs::read_to_string(path).map_err(anyhow::Error::from),
        )
        .expect("default file should be loaded");

        assert_eq!(pem, "PEM_FROM_DEFAULT_FILE");
        if let Err(error) = fs::remove_file(&key_path) {
            eprintln!("[runtime test] failed to remove key file: {error}");
        }
        if let Err(error) = fs::remove_dir_all(home) {
            eprintln!("[runtime test] failed to remove temp home dir: {error}");
        }
    }

    #[test]
    fn cli_parser_accepts_build_frontend_flag() {
        let options =
            parse_cli_args_from(["commut", BUILD_FRONTEND_ARG]).expect("flag should parse");

        assert!(options.build_frontend);
    }

    #[test]
    fn cli_parser_rejects_unknown_arguments() {
        let error = parse_cli_args_from(["commut", "--wat"]).expect_err("unknown args must fail");
        assert!(error.to_string().contains("unknown argument"));
    }

    #[test]
    fn build_frontend_assets_returns_canonicalized_out_link_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let temp = std::env::temp_dir().join(format!("commut-build-frontend-test-{unique}"));
        let out_link = temp.join("result");
        let target = temp.join("store-output");
        fs::create_dir_all(&target).expect("target dir should exist");

        let resolved = build_frontend_assets_with(Path::new("/tmp"), &out_link, |_, out_link| {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, out_link).map_err(anyhow::Error::from)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&target, out_link).map_err(anyhow::Error::from)?;
            Ok(())
        })
        .expect("out link should resolve");

        assert_eq!(
            resolved,
            fs::canonicalize(&target).expect("canonical target")
        );
        if let Err(error) = fs::remove_file(&out_link) {
            eprintln!("[runtime test] failed to remove out link: {error}");
        }
        if let Err(error) = fs::remove_dir_all(&temp) {
            eprintln!("[runtime test] failed to remove temp dir: {error}");
        }
    }

    #[test]
    fn debug_runs_build_frontend_when_static_dirs_are_missing() {
        let should_build = should_build_frontend_for_run(CliOptions::default(), false, false);
        assert!(should_build);
    }

    #[test]
    fn explicit_static_dirs_skip_default_dev_build() {
        let should_build = should_build_frontend_for_run(CliOptions::default(), true, true);
        assert!(!should_build);
    }

    #[test]
    fn explicit_flag_overrides_static_dir_presence() {
        let should_build = should_build_frontend_for_run(
            CliOptions {
                build_frontend: true,
            },
            true,
            true,
        );
        assert!(should_build);
    }
}
