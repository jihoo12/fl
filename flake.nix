{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    naersk,
    fenix,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      rustToolchain = fenix.packages.${system}.latest.toolchain;

      naersk' = naersk.lib.${system}.override {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
    in {
      packages.default = naersk'.buildPackage {
        src = ./.;
        name = "fl";
      };

      apps.default = flake-utils.lib.mkApp {
        drv = self.packages.${system}.default;
      };

      devShells.default = pkgs.mkShell {
        packages = [
          rustToolchain
        ];
      };
    });
}
