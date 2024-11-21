{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    { nixpkgs, fenix, ... }:
    {
      packages = builtins.mapAttrs (
        system: fenix:
        let
          manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
        in
        {
          ${manifest.name} = (
            (nixpkgs.legacyPackages.${system}.makeRustPlatform (
              let
                toolchain = fenix.stable.toolchain;
              in
              {
                cargo = toolchain;
                rustc = toolchain;
              }
            )).buildRustPackage
              {
                pname = manifest.name;
                version = manifest.version;
                src = ./.;
                cargoLock.lockFile = ./Cargo.lock;
                ABBREVIATIONS_JSON = nixpkgs.legacyPackages.${system}.fetchurl {
                  url = "https://raw.githubusercontent.com/leanprover/vscode-lean4/refs/tags/v0.0.184/lean4-unicode-input/src/abbreviations.json";
                  hash = "sha256-dJtxx+zt0td3CX8+NQHLPa1EsTjvz+QLSoq7yP2s2u0=";
                };
              }
          );
        }
      ) fenix.packages;
    };
}
