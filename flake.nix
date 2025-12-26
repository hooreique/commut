{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    uno-ls = {
      url = "github:hooreique/unocss-language-server";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = inputs: inputs.flake-utils.lib.eachDefaultSystem (system: let
    pkgs = inputs.nixpkgs.legacyPackages.${system} // {
      unocss-language-server = inputs.uno-ls.packages.${system}.default;
    };
  in {
    devShells.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        nodejs_22
        pnpm
        typescript
        typescript-language-server
        unocss-language-server
      ];
    };
  });
}
