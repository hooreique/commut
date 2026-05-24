{
  clientPackage,
  lib,
  makeWrapper,
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

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  cargoBuildFlags = [
    "--bin"
    "commut"
  ];
  doCheck = false;

  preBuild = ''
    export RUSTFLAGS="--remap-path-prefix=$NIX_BUILD_TOP=/build ''${RUSTFLAGS:-}"
  '';

  nativeBuildInputs = [
    makeWrapper
    pkg-config
  ];

  postInstall = ''
    mv $out/bin/commut $out/bin/.commut-wrapped
    makeWrapper $out/bin/.commut-wrapped $out/bin/commut \
      --set COMMUT_PUBLIC_DIR ${clientPackage}/public \
      --set COMMUT_BUILD_DIR ${clientPackage}/build \
      --set COMMUT_DIST_DIR ${clientPackage}/dist
  '';

  meta = {
    mainProgram = pname;
    description = "commut server";
    homepage = "https://github.com/hooreique/commut";
  };
}
