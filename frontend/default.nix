{
  fetchurl,
  gnutar,
  lib,
  nodejs_22,
  pnpm_9,
  runCommand,
  stdenvNoCC,
  woff2,
  xz,
}:

let
  hackArchive = fetchurl {
    url = "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.4.0/Hack.tar.xz";
    hash = "sha256-HQChQ1Y4CEF0UWl1hAhUNopFrDC7C60sDEnbcTtZJfA=";
  };

  hackFonts =
    runCommand "commut-hack-nerd-fonts-3.4.0"
      {
        nativeBuildInputs = [
          gnutar
          woff2
          xz
        ];
      }
      ''
        mkdir -p "$out/fonts" source
        tar -xJf ${hackArchive} -C source

        for style in Regular Bold Italic BoldItalic; do
          cp "source/HackNerdFont-$style.ttf" "$out/fonts/"
          woff2_compress "$out/fonts/HackNerdFont-$style.ttf" >/dev/null
          rm -f "$out/fonts/HackNerdFont-$style.ttf"
        done
      '';
in
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "commut-client";
  version = "0.1.0";

  src = lib.cleanSource ./.;

  prePnpmInstall = "";

  nativeBuildInputs = [
    nodejs_22
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
    hash = "sha256-4wn2gDUKYEDX0kCfRJ0uAaKp2qkUjOZEywt9npSCU04=";
  };

  buildPhase = ''
    runHook preBuild
    pnpm run prepare
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/public/fonts
    cp -R build public $out/
    cp -R ${hackFonts}/fonts/. $out/public/fonts/
    runHook postInstall
  '';
})
