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
        client = pkgs.callPackage ./frontend/default.nix { };
        commut = pkgs.callPackage ./backend/default.nix {
          clientPackage = client;
        };
        installer = pkgs.callPackage ./installer/default.nix {
          backendPackage = commut;
          clientPackage = client;
          installFlakeRef = installFlakeRef;
        };
      in
      {
        packages = {
          "commut-client" = client;
          commut = commut;
          default = commut;
          "commut-installer" = installer;
        };

        apps."commut-installer" = {
          type = "app";
          program = "${installer}/bin/commut-installer";
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.nodejs_22
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
