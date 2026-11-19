{
  lib,
  rustPlatform,
}:
let
  original = lib.importTOML ./Cargo.toml;
  pname = "commut-server";
  version = original.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version;

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  cargoBuildFlags = [
    "--bin"
    "commut-server"
  ];

  doCheck = false;

  preBuild = ''
    export RUSTFLAGS="--remap-path-prefix=$NIX_BUILD_TOP=/build ''${RUSTFLAGS:-}"
  '';

  meta = {
    mainProgram = "commut-server";
    description = "commut server";
    homepage = "https://github.com/hooreique/commut";
  };
}
