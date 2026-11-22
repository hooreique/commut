{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.commut;
  packages = self.packages.${pkgs.stdenv.hostPlatform.system};
in
{
  options.services.commut = {
    enable = lib.mkEnableOption "commut";
    domain = lib.mkOption {
      type = lib.types.strMatching "[a-zA-Z0-9][a-zA-Z0-9.-]*";
      description = "Domain serving the commut browser client.";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "Loopback port for the commut backend.";
    };
  };

  config = lib.mkIf cfg.enable {
    hooddy.sites.commut.config =
      builtins.replaceStrings
        [ "$domain" "$client_root" "$upstream" ]
        [ cfg.domain "${packages.commut-client}" "127.0.0.1:${toString cfg.port}" ]
        (builtins.readFile ./installer/Caddyfile.template);
    hooddy.services.commut.unit = ''
      [Unit]
      Description=Commut
      After=network.target

      [Service]
      Environment=COMMUT_HOST=127.0.0.1
      Environment=COMMUT_PORT=${toString cfg.port}
      Environment=COMMUT_AUTHORIZED_PUBLIC_KEY_PEM_FILE=%h/.config/commut/authorized.pub.pem
      ExecStart=${packages.commut-server}/bin/commut-server
      Restart=on-failure
      RestartSec=2s

      [Install]
      WantedBy=default.target
    '';
  };
}
