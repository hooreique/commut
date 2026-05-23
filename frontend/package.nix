{
  fonts,
  lib,
  nodejs_24,
  pnpm_9,
  stdenvNoCC,
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "commut-client";
  version = "0.1.0";

  src = lib.cleanSource ./.;

  prePnpmInstall = "";

  nativeBuildInputs = [
    nodejs_24
    pnpm_9.configHook
  ];

  pnpmDeps = pnpm_9.fetchDeps {
    inherit (finalAttrs)
      pname
      version
      src
      prePnpmInstall
      ;
    fetcherVersion = 2;
    hash = "sha256-3dWlALAtoSfp4IVV7ItAexcVxPAWXQBacZ0mq+slruA=";
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
})
