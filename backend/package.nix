{
  clientPackage,
  lib,
  makeWrapper,
  pkg-config,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "commut";
  version = "0.1.0";

  src = lib.cleanSource ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  cargoBuildFlags = [
    "--bin"
    "commut"
  ];
  doCheck = false;

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
    mainProgram = "commut";
    description = "commut server";
  };
}
