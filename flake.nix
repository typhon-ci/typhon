{
  description = "Typhon";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, flake-utils, nixpkgs, crane, rust-overlay }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        typhon = let craneLib = crane.lib.${system};
        in craneLib.buildPackage {
          name = "typhon";
          buildInputs = [ pkgs.sqlite.dev ];
          src = craneLib.cleanCargoSource ./.;
        };
        typhon-webapp = let
          rust-wasm = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "wasm32-unknown-unknown" ];
          };
          craneLibWasm = (crane.mkLib pkgs).overrideToolchain rust-wasm;
        in craneLibWasm.buildPackage {
          name = "typhon-webapp";
          buildInputs =
            [ pkgs.pkg-config pkgs.openssl.dev pkgs.nodePackages.sass ];
          src = craneLibWasm.cleanCargoSource ./.;
          cargoBuildCommand = "cargo build -p typhon-webapp";
        };
        common-devShell-packages = [ pkgs.rustfmt ];
      in {
        packages = {
          inherit typhon;
          default = typhon;
        };
        devShells = {
          default = pkgs.mkShell {
            name = "typhon-shell";
            packages = [ pkgs.diesel-cli pkgs.sqlite ];
            inputsFrom = [ typhon ] ++ common-devShell-packages;
            DATABASE_URL = "sqlite:typhon.sqlite";
          };
          typhon-webapp = pkgs.mkShell {
            name = "typhon-webapp-shell";
            packages = [ ] ++ common-devShell-packages;
            inputsFrom = [ typhon-webapp ];
          };
        };
      });
}
