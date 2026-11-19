{ backendPackage, clientPackage, python3, makeWrapper, symlinkJoin, nix, systemd, stdenv, lib }:
let
  script = python3.pkgs.buildPythonApplication {
    pname = "commut-cli";
    version = "0.1.0";
    format = "other";
    src = ./.;
    dontBuild = true;
    installPhase = ''
      mkdir -p $out/bin
      cp commut.py $out/bin/commut
      chmod +x $out/bin/commut
      substituteInPlace $out/bin/commut \
        --replace-fail '@server@' '${backendPackage}' \
        --replace-fail '@client@' '${clientPackage}' \
        --replace-fail '@template@' '${../installer/Caddyfile.template}'
    '';
  };
in symlinkJoin {
  name = "commut";
  paths = [ script ];
  nativeBuildInputs = [ makeWrapper ];
  postBuild = ''
    wrapProgram $out/bin/commut --prefix PATH : ${lib.makeBinPath ([ nix ] ++ lib.optionals stdenv.isLinux [ systemd ])}
  '';
  meta.mainProgram = "commut";
}
