{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    uno-ls.url = "github:hooreique/unocss-language-server";
  };

  outputs = inputs: let
    eachSystem = systems: f: builtins.foldl' (attrs: system: let
      ret = f system;
    in builtins.foldl' (attrs: key: attrs // {
      ${key} = (attrs.${key} or {}) // { ${system} = ret.${key}; };
    }) attrs (builtins.attrNames ret)) {} systems;
  in eachSystem [ "aarch64-linux" "x86_64-linux" "aarch64-darwin" ] (system: let
    pkgs = inputs.nixpkgs.legacyPackages.${system};
  in {
    devShells.default = pkgs.mkShell {
      packages = [
        pkgs.nodejs_22
        pkgs.pnpm
        pkgs.typescript
        pkgs.typescript-language-server
        inputs.uno-ls.packages.${system}.default
      ];
    };
  });
}
