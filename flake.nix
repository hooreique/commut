{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    uno-ls.url = "github:hooreique/unocss-language-server";
  };

  outputs =
    inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = inputs.nixpkgs.legacyPackages.${system};
        flakeRef = "github:hooreique/commut";
        installFlakeRef = "${flakeRef}#commut-installer";
        fonts = pkgs.callPackage ./hack-woff2.nix { };
        prepare-fonts = pkgs.callPackage ./prepare-fonts.nix {
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

        apps.prepare-fonts = inputs.flake-utils.lib.mkApp {
          drv = prepare-fonts;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.nodejs_24
            pkgs.pnpm
            pkgs.typescript
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
      }
    );
}
