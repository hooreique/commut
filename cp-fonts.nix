{
  coreutils,
  fonts,
  writeShellApplication,
}:

writeShellApplication {
  name = "cp-fonts";
  runtimeInputs = [ coreutils ];
  text = ''
    if [ "$#" -ne 1 ]; then
      echo "usage: cp-fonts <output-dir>" >&2
      exit 64
    fi

    target=$1
    if [ -e "$target" ] && [ ! -d "$target" ]; then
      echo "cp-fonts: target exists but is not a directory: $target" >&2
      exit 1
    fi

    mkdir -p "$target"
    font_dir=${fonts}/share/fonts
    cp -f "$font_dir"/*.woff2 "$target"/
  '';
}
