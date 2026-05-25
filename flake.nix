{
  description = "commut";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    uno-ls.url = "github:hooreique/unocss-language-server";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      perSystem =
        { system, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (final: prev: { pnpm = prev.pnpm.override { nodejs = final.nodejs_26; }; })
            ];
          };
          flakeRef = "github:hooreique/commut";
          installFlakeRef = "${flakeRef}#commut-installer";
          fonts = pkgs.callPackage ./hack-woff2.nix { };
          cp-fonts = pkgs.callPackage ./cp-fonts.nix {
            inherit fonts;
          };
          devServerDeps = [
            pkgs.caddy
            pkgs.cargo
            pkgs.coreutils
            pkgs.nodejs_26
            pkgs.pkg-config
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
          commut = pkgs.callPackage ./backend/package.nix { };
          installer = pkgs.callPackage ./installer/package.nix {
            backendPackage = commut;
            clientPackage = client;
            installFlakeRef = installFlakeRef;
          };
        in
        {
          packages = {
            default = commut;
            commut = commut;
            commut-client = client;
            commut-installer = installer;
          };

          apps.cp-fonts.program = cp-fonts;
          apps.dev-server.program = dev-server;

          devShells.default = pkgs.mkShell {
            packages = devServerDeps ++ [
              pkgs.typescript-language-server
              pkgs.clippy
              pkgs.rust-analyzer
              pkgs.rustfmt
              pkgs.woff2
              inputs.uno-ls.packages.${system}.default
            ];
          };
        };
    };
}
