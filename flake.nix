{
  description = "Typhon";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.follows = "crane/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, crane, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        craneLib = crane.lib.${system};
        rust-wasm = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain rust-wasm;
      in rec {
        packages = {
          typhon-webapp = craneLibWasm.buildPackage {
            name = "typhon-webapp";
            buildInputs = [
              pkgs.pkg-config pkgs.openssl.dev
              pkgs.nodePackages.sass
            ];
            src = craneLib.cleanCargoSource ./.;
            cargoBuildCommand = "cargo build -p typhon-webapp";
          };
          typhon-server = craneLib.buildPackage {
            name = "typhon";
            buildInputs = [ pkgs.sqlite.dev ];
            src = craneLib.cleanCargoSource ./.;
          };
          default = packages.typhon-server;
        };
        
        devShells = {
          default = devShells.typhon-server;
          typhon-server = pkgs.mkShell {
            packages = [
              pkgs.diesel-cli
              pkgs.sqlite
              pkgs.sqlitebrowser
              pkgs.rustfmt
            ];
            inputsFrom = [ packages.typhon-server ];
            DATABASE_URL = "sqlite:typhon.sqlite";
          };
          typhon-webapp = pkgs.mkShell {
            packages = [ pkgs.rustfmt ];
            inputsFrom = [ packages.typhon-webapp ];
          };
        };
        
        nixosModules.default = import ./nixos/typhon.nix packages.typhon-server;
        # checks.default = import ./nixos/test.nix {
        #   inherit system nixpkgs;
        #   typhon = self;
        # };
      }
    );
}
