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

use crate::app::StaticAssetRoots;

pub const DEFAULT_AUTHORIZED_PUBLIC_KEY_RELATIVE_PATH: &str = ".config/commut/authorized.pub.pem";
const REMOVED_BUILD_FRONTEND_ARG: &str = "--build-frontend";
pub const COMMUT_PUBLIC_DIR_ENV: &str = "COMMUT_PUBLIC_DIR";
pub const COMMUT_BUILD_DIR_ENV: &str = "COMMUT_BUILD_DIR";
pub const COMMUT_DIST_DIR_ENV: &str = "COMMUT_DIST_DIR";
pub const COMMUT_LEGACY_PAGES_DIR_ENV: &str = "COMMUT_PAGES_DIR";
const FRONTEND_ASSET_USAGE: &str = "Build frontend assets before running the backend:\n  pnpm --dir frontend install\n  pnpm --dir frontend run build";
const FRONTEND_FONT_USAGE: &str = "Prepare frontend fonts before running the backend:\n  nix run .#prepare-fonts frontend/public/fonts";

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

/// Resolve and validate frontend static asset directories for the executable.
///
/// # Errors
///
/// Returns an error when one of the configured asset directories does not
/// exist, or when the public fonts directory has no `.woff2` files.
pub fn load_static_asset_roots() -> Result<StaticAssetRoots> {
    load_static_asset_roots_with(
        |name| env::var_os(name).map(PathBuf::from),
        |path| path.is_dir(),
        contains_woff2_file,
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
        if arg == REMOVED_BUILD_FRONTEND_ARG {
            return Err(anyhow!(
                "`{REMOVED_BUILD_FRONTEND_ARG}` is no longer supported.\n\n{FRONTEND_ASSET_USAGE}"
            ));
        }

        return Err(anyhow!(
            "unknown argument: {}",
            PathBuf::from(&arg).display()
        ));
    }

    Ok(())
}

fn load_static_asset_roots_with<GetEnvPath, IsDir, HasWoff2>(
    get_env_path: GetEnvPath,
    is_dir: IsDir,
    has_woff2_file: HasWoff2,
) -> Result<StaticAssetRoots>
where
    GetEnvPath: Fn(&str) -> Option<PathBuf>,
    IsDir: Fn(&Path) -> bool,
    HasWoff2: Fn(&Path) -> bool,
{
    let roots = StaticAssetRoots::with_overrides(
        non_empty_env_path(&get_env_path, COMMUT_PUBLIC_DIR_ENV),
        non_empty_env_path(&get_env_path, COMMUT_BUILD_DIR_ENV)
            .or_else(|| non_empty_env_path(&get_env_path, COMMUT_LEGACY_PAGES_DIR_ENV)),
        non_empty_env_path(&get_env_path, COMMUT_DIST_DIR_ENV),
    );
    validate_static_asset_roots(&roots, is_dir, has_woff2_file)?;
    Ok(roots)
}

fn non_empty_env_path<GetEnvPath>(get_env_path: &GetEnvPath, name: &str) -> Option<PathBuf>
where
    GetEnvPath: Fn(&str) -> Option<PathBuf>,
{
    get_env_path(name).filter(|path| !path.as_os_str().is_empty())
}

fn validate_static_asset_roots<IsDir, HasWoff2>(
    roots: &StaticAssetRoots,
    is_dir: IsDir,
    has_woff2_file: HasWoff2,
) -> Result<()>
where
    IsDir: Fn(&Path) -> bool,
    HasWoff2: Fn(&Path) -> bool,
{
    let asset_dirs = [
        ("public", COMMUT_PUBLIC_DIR_ENV, roots.public_dir.as_path()),
        ("build", COMMUT_BUILD_DIR_ENV, roots.pages_dir.as_path()),
        ("dist", COMMUT_DIST_DIR_ENV, roots.dist_dir.as_path()),
    ];
    let missing = asset_dirs
        .into_iter()
        .filter(|(_, _, path)| !is_dir(path))
        .map(|(label, env_var, path)| format!("  - {label} ({env_var}): {}", path.display()))
        .collect::<Vec<_>>();

    if missing.is_empty() {
        let fonts_dir = roots.public_dir.join("fonts");
        if is_dir(&fonts_dir) && has_woff2_file(&fonts_dir) {
            return Ok(());
        }

        return Err(anyhow!(
            "frontend fonts are not ready.\n\nExpected at least one .woff2 file under:\n  {}\n\n{}",
            fonts_dir.display(),
            FRONTEND_FONT_USAGE
        ));
    }

    Err(anyhow!(
        "frontend assets are not ready.\n\nMissing directories:\n{}\n\n{}\n\nThe backend reads these environment variables when they are set:\n  COMMUT_PUBLIC_DIR=frontend/public\n  COMMUT_BUILD_DIR=frontend/build\n  COMMUT_DIST_DIR=frontend/dist",
        missing.join("\n"),
        FRONTEND_ASSET_USAGE
    ))
}

fn contains_woff2_file(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(std::result::Result::ok)
        .any(|entry| {
            let is_woff2 = entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("woff2");
            let is_file = entry.file_type().is_ok_and(|file_type| file_type.is_file());
            is_woff2 && is_file
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
    fn cli_parser_accepts_no_arguments() {
        parse_cli_args_from(["commut"]).expect("empty args should parse");
    }

    #[test]
    fn cli_parser_rejects_removed_build_frontend_flag_with_frontend_guidance() {
        let error = parse_cli_args_from(["commut", REMOVED_BUILD_FRONTEND_ARG])
            .expect_err("removed build flag must fail");
        let message = error.to_string();
        assert!(message.contains("no longer supported"));
        assert!(message.contains("pnpm --dir frontend install"));
        assert!(message.contains("pnpm --dir frontend run build"));
    }

    #[test]
    fn cli_parser_rejects_unknown_arguments() {
        let error = parse_cli_args_from(["commut", "--wat"]).expect_err("unknown args must fail");
        assert!(error.to_string().contains("unknown argument"));
    }

    #[test]
    fn static_asset_roots_use_env_overrides() {
        let roots = load_static_asset_roots_with(
            |name| match name {
                COMMUT_PUBLIC_DIR_ENV => Some(PathBuf::from("/assets/public")),
                COMMUT_BUILD_DIR_ENV => Some(PathBuf::from("/assets/build")),
                COMMUT_DIST_DIR_ENV => Some(PathBuf::from("/assets/dist")),
                _ => None,
            },
            |path| {
                [
                    Path::new("/assets/public"),
                    Path::new("/assets/public/fonts"),
                    Path::new("/assets/build"),
                    Path::new("/assets/dist"),
                ]
                .contains(&path)
            },
            |_| true,
        )
        .expect("configured asset roots should load");

        assert_eq!(roots.public_dir, PathBuf::from("/assets/public"));
        assert_eq!(roots.pages_dir, PathBuf::from("/assets/build"));
        assert_eq!(roots.dist_dir, PathBuf::from("/assets/dist"));
    }

    #[test]
    fn static_asset_roots_fall_back_to_repo_layout() {
        let roots = load_static_asset_roots_with(|_| None, |_| true, |_| true)
            .expect("repository defaults should load when dirs exist");

        assert!(roots.public_dir.ends_with(Path::new("frontend/public")));
        assert!(roots.pages_dir.ends_with(Path::new("frontend/build")));
        assert!(roots.dist_dir.ends_with(Path::new("frontend/dist")));
    }

    #[test]
    fn static_asset_roots_accept_legacy_pages_env() {
        let roots = load_static_asset_roots_with(
            |name| match name {
                COMMUT_PUBLIC_DIR_ENV => Some(PathBuf::from("/assets/public")),
                COMMUT_LEGACY_PAGES_DIR_ENV => Some(PathBuf::from("/legacy/build")),
                COMMUT_DIST_DIR_ENV => Some(PathBuf::from("/assets/dist")),
                _ => None,
            },
            |path| {
                [
                    Path::new("/assets/public"),
                    Path::new("/assets/public/fonts"),
                    Path::new("/legacy/build"),
                    Path::new("/assets/dist"),
                ]
                .contains(&path)
            },
            |_| true,
        )
        .expect("legacy pages env should load");

        assert_eq!(roots.pages_dir, PathBuf::from("/legacy/build"));
    }

    #[test]
    fn static_asset_roots_report_missing_dirs_with_build_commands() {
        let error = load_static_asset_roots_with(
            |name| match name {
                COMMUT_PUBLIC_DIR_ENV => Some(PathBuf::from("/missing/public")),
                COMMUT_BUILD_DIR_ENV => Some(PathBuf::from("/missing/build")),
                COMMUT_DIST_DIR_ENV => Some(PathBuf::from("/missing/dist")),
                _ => None,
            },
            |_| false,
            |_| false,
        )
        .expect_err("missing asset roots must fail");
        let message = error.to_string();

        assert!(message.contains("frontend assets are not ready"));
        assert!(message.contains("public (COMMUT_PUBLIC_DIR): /missing/public"));
        assert!(message.contains("build (COMMUT_BUILD_DIR): /missing/build"));
        assert!(message.contains("dist (COMMUT_DIST_DIR): /missing/dist"));
        assert!(message.contains("pnpm --dir frontend install"));
        assert!(message.contains("pnpm --dir frontend run build"));
    }

    #[test]
    fn static_asset_roots_report_missing_fonts_with_prepare_command() {
        let error = load_static_asset_roots_with(
            |name| match name {
                COMMUT_PUBLIC_DIR_ENV => Some(PathBuf::from("/assets/public")),
                COMMUT_BUILD_DIR_ENV => Some(PathBuf::from("/assets/build")),
                COMMUT_DIST_DIR_ENV => Some(PathBuf::from("/assets/dist")),
                _ => None,
            },
            |_| true,
            |_| false,
        )
        .expect_err("missing fonts must fail");
        let message = error.to_string();

        assert!(message.contains("frontend fonts are not ready"));
        assert!(message.contains("/assets/public/fonts"));
        assert!(message.contains(".woff2"));
        assert!(message.contains("nix run .#prepare-fonts frontend/public/fonts"));
    }

    #[test]
    fn font_check_accepts_any_woff2_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let fonts_dir = std::env::temp_dir().join(format!("commut-font-check-test-{unique}"));
        fs::create_dir_all(&fonts_dir).expect("font test dir should be created");
        fs::write(fonts_dir.join("custom-terminal-font.woff2"), "font")
            .expect("test font should be written");

        assert!(contains_woff2_file(&fonts_dir));

        if let Err(error) = fs::remove_dir_all(fonts_dir) {
            eprintln!("[runtime test] failed to remove temp fonts dir: {error}");
        }
    }
}
