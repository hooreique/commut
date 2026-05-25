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
};

use anyhow::{Context, Result, anyhow};

pub const DEFAULT_AUTHORIZED_PUBLIC_KEY_RELATIVE_PATH: &str = ".config/commut/authorized.pub.pem";

/// Parse supported CLI flags for the backend binary.
///
/// # Errors
///
/// Returns an error when an unsupported CLI flag is supplied.
pub fn parse_cli_args() -> Result<()> {
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

fn parse_cli_args_from<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    if let Some(arg) = args.into_iter().nth(1) {
        let arg = arg.into();
        return Err(anyhow!(
            "unknown argument: {}",
            PathBuf::from(&arg).display()
        ));
    }

    Ok(())
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
    fn cli_parser_accepts_no_arguments() {
        parse_cli_args_from(["commut"]).expect("empty args should parse");
    }

    #[test]
    fn cli_parser_rejects_unknown_arguments() {
        let error = parse_cli_args_from(["commut", "--wat"]).expect_err("unknown args must fail");
        assert!(error.to_string().contains("unknown argument"));
    }
}
