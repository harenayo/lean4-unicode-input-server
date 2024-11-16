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
          ${manifest.name} =
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
              };
        }
      ) fenix.packages;
    };
}
