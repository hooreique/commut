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
          client = pkgs.callPackage ./frontend/package.nix {
            inherit fonts;
          };
          commut = pkgs.callPackage ./backend/package.nix {
            clientPackage = client;
          };
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
            commut-installer = installer;
          };

          apps.cp-fonts.program = cp-fonts;

          devShells.default = pkgs.mkShell {
            packages = [
              pkgs.nodejs_26
              pkgs.pnpm
              pkgs.typescript-language-server
              pkgs.cargo
              pkgs.clippy
              pkgs.pkg-config
              pkgs.rust-analyzer
              pkgs.rustc
              pkgs.rustfmt
              pkgs.caddy
              pkgs.woff2
              inputs.uno-ls.packages.${system}.default
            ];
          };
        };
    };
}
