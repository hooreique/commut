use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use sha2::{Digest, Sha256};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by Cargo"),
    );
    let digest = backend_source_digest(&manifest_dir)
        .expect("backend source digest should be computable at build time");

    println!("cargo:rustc-env=COMMUT_BACKEND_SOURCE_DIGEST={digest}");
}

fn backend_source_digest(manifest_dir: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();

    for path in backend_source_files(manifest_dir)? {
        let relative_path = normalized_relative_path(&path, manifest_dir);
        let contents = fs::read(&path)?;

        hasher.update(relative_path.as_bytes());
        hasher.update([0]);
        hasher.update(contents.len().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(contents);
        hasher.update([0]);
    }

    Ok(BASE64_STANDARD.encode(hasher.finalize()))
}

fn backend_source_files(manifest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = vec![
        manifest_dir.join("Cargo.toml"),
        manifest_dir.join("Cargo.lock"),
        manifest_dir.join("build.rs"),
    ];
    collect_rust_sources(&manifest_dir.join("src"), &mut files)?;
    files.sort_by_key(|path| normalized_relative_path(path, manifest_dir));
    Ok(files)
}

fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_rust_sources(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }

    Ok(())
}

fn normalized_relative_path(path: &Path, manifest_dir: &Path) -> String {
    path.strip_prefix(manifest_dir)
        .expect("source path should be inside backend crate")
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
