{
  coreutils,
  fonts,
  writeShellApplication,
}:

writeShellApplication {
  name = "prepare-fonts";
  runtimeInputs = [ coreutils ];
  text = ''
    if [ "$#" -ne 1 ]; then
      echo "usage: prepare-fonts <output-dir>" >&2
      exit 64
    fi

    target=$1
    if [ -e "$target" ] && [ ! -d "$target" ]; then
      echo "prepare-fonts: target exists but is not a directory: $target" >&2
      exit 1
    fi

    mkdir -p "$target"
    font_dir=${fonts}/share/fonts
    cp -f "$font_dir"/*.woff2 "$target"/
  '';
}
