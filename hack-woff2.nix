{
  fetchurl,
  gnutar,
  runCommand,
  woff2,
  xz,
}:

let
  hackArchive = fetchurl {
    url = "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.4.0/Hack.tar.xz";
    hash = "sha256-HQChQ1Y4CEF0UWl1hAhUNopFrDC7C60sDEnbcTtZJfA=";
  };
in
runCommand "commut-hack-nerd-fonts-3.4.0"
  {
    nativeBuildInputs = [
      gnutar
      woff2
      xz
    ];
  }
  ''
    mkdir -p "$out/share/fonts" source
    tar -xJf ${hackArchive} -C source

    for style in Regular Bold Italic BoldItalic; do
      cp "source/HackNerdFont-$style.ttf" "$out/share/fonts/"
      woff2_compress "$out/share/fonts/HackNerdFont-$style.ttf" >/dev/null
      rm -f "$out/share/fonts/HackNerdFont-$style.ttf"
    done
  ''
