{
  description = "commut";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    uno-ls.url = "github:xna00/unocss-language-server";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      flake.hooddyModules.default =
        { config, lib, pkgs, ... }:
        let
          cfg = config.services.commut;
          packages = inputs.self.packages.${pkgs.stdenv.hostPlatform.system};
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
            hooddy.sites.commut.config = builtins.replaceStrings
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
        };

      perSystem =
        { system, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (final: prev: { pnpm = prev.pnpm.override { nodejs = final.nodejs_26; }; })
            ];
          };
          fonts = pkgs.callPackage ./hack-woff2.nix { };
          cp-fonts = pkgs.callPackage ./cp-fonts.nix {
            inherit fonts;
          };
          devServerDeps = [
            pkgs.caddy
            pkgs.cargo
            pkgs.coreutils
            pkgs.nodejs_26
            pkgs.pnpm
            pkgs.rustc
          ];
          dev-server = pkgs.callPackage ./dev-server.nix {
            inherit fonts;
            runtimeInputs = devServerDeps;
          };
          client = pkgs.callPackage ./frontend/package.nix {
            inherit fonts;
          };
          server = pkgs.callPackage ./backend/package.nix { };
          commut = pkgs.callPackage ./cli/package.nix {
            backendPackage = server;
            clientPackage = client;
          };
        in
        {
          packages = {
            default = commut;
            commut = commut;
            commut-client = client;
            commut-server = server;
          };

          apps.default.program = "${commut}/bin/commut";
          apps.commut.program = "${commut}/bin/commut";
          apps.cp-fonts.program = cp-fonts;
          apps.dev-server.program = dev-server;

          devShells.default = pkgs.mkShell {
            packages = devServerDeps ++ [
              pkgs.typescript-language-server
              pkgs.clippy
              pkgs.rust-analyzer
              pkgs.rustfmt
              pkgs.python3
              pkgs.woff2
              inputs.uno-ls.packages.${system}.default
            ];
          };
        };
    };
}
