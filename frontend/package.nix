{
  lib,
  stdenvNoCC,
  fonts,
  nodejs_26,
  pnpm,
  pnpmConfigHook,
  fetchPnpmDeps,
}:
let
  original = builtins.fromJSON (builtins.readFile ./package.json);
  pname = original.name;
  version = original.version;
in
stdenvNoCC.mkDerivation (finalAttrs: {
  inherit pname version;

  src = lib.cleanSource ./.;

  prePnpmInstall = "";

  nativeBuildInputs = [
    nodejs_26
    pnpm
    pnpmConfigHook
  ];

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs)
      pname
      version
      src
      prePnpmInstall
      ;
    pnpm = pnpm;
    fetcherVersion = 3;
    hash = "sha256-WVlIp0akIY6VDlG4IjtZkHjPSAieQc0Dza8f5KKj374=";
  };

  buildPhase = ''
    runHook preBuild
    pnpm run build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/public/fonts
    cp -R build dist public $out/
    cp -R ${fonts}/share/fonts/. $out/public/fonts/
    runHook postInstall
  '';

  meta = {
    description = "commut client";
    homepage = "https://github.com/hooreique/commut";
  };
})
