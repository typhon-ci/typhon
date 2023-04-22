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
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        src = pkgs.lib.sourceByRegex ./. [
          "Cargo.toml"
          "Cargo.lock"
          "typhon.*"
          "typhon-types.*"
          "typhon-webapp.*"
        ];
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLib = crane.lib.${system}.overrideToolchain rustToolchain;
        typhon = let cargoArtifacts = craneLib.buildDepsOnly { inherit src; };
        in craneLib.buildPackage {
          name = "typhon";
          inherit src cargoArtifacts;
          buildInputs = [ pkgs.sqlite.dev ];
        };
        typhon-webapp = let
          cargoExtraArgs = "-p typhon-webapp --target wasm32-unknown-unknown";
          CARGO_PROFILE = "typhon-webapp";
          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src cargoExtraArgs CARGO_PROFILE;
            doCheck = false;
          };
          wasm = craneLib.buildPackage {
            name = "typhon-webapp-wasm";
            inherit src cargoExtraArgs CARGO_PROFILE cargoArtifacts;
            doCheck = false;
          };
        in pkgs.stdenv.mkDerivation {
          name = "typhon-webapp";
          phases = [ "buildPhase" ];
          nativeBuildInputs = [ pkgs.wasm-bindgen-cli pkgs.binaryen ];
          buildPhase = ''
            wasm-opt -o typhon_webapp.wasm ${wasm}/lib/typhon_webapp.wasm
            wasm-bindgen --out-dir $out --target web typhon_webapp.wasm
          '';
        };
        typhon-doc = pkgs.stdenv.mkDerivation {
          name = "typhon-doc";
          src = ./doc;
          nativeBuildInputs = [ pkgs.mdbook ];
          buildPhase = "mdbook build";
          installPhase = "cp -r book $out";
        };
      in {
        packages = {
          inherit typhon typhon-webapp typhon-doc;
          default = typhon;
        };
        devShells.default = pkgs.mkShell {
          name = "typhon-shell";
          packages = [
            # Rust
            pkgs.rustfmt
            pkgs.rust-analyzer
            rustToolchain

            # Typhon server
            pkgs.bubblewrap
            pkgs.diesel-cli
            pkgs.pkg-config
            pkgs.sqlite

            # Typhon webapp
            pkgs.nodePackages.sass
            pkgs.trunk

            # Documentation
            pkgs.mdbook
          ];
          DATABASE_URL = "sqlite:typhon.sqlite";
        };
        checks.default = import ./nixos/test.nix {
          inherit system nixpkgs;
          typhon = self;
        };
        actions = import ./actions { inherit pkgs; };
      }) // {
        nixosModules.default = import ./nixos/typhon.nix self;
      };
}
