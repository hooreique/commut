{
  lib,
  pkg-config,
  rustPlatform,
}:
let
  original = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  pname = original.package.name;
  version = original.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version;

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  cargoBuildFlags = [ "--bin" "commut" ];

  doCheck = false;

  preBuild = ''
    export RUSTFLAGS="--remap-path-prefix=$NIX_BUILD_TOP=/build ''${RUSTFLAGS:-}"
  '';

  nativeBuildInputs = [
    pkg-config
  ];

  meta = {
    mainProgram = pname;
    description = "commut server";
    homepage = "https://github.com/hooreique/commut";
  };
}
