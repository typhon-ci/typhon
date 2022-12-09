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
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            path == toString ./Cargo.toml || path == toString ./Cargo.lock
            || pkgs.lib.hasPrefix (toString ./typhon) path
            || pkgs.lib.hasPrefix (toString ./typhon-types) path
            || pkgs.lib.hasPrefix (toString ./typhon-webapp) path;
        };
        typhon = let
          craneLib = crane.lib.${system};
          cargoArtifacts = craneLib.buildDepsOnly { inherit src; };
        in craneLib.buildPackage {
          name = "typhon";
          inherit src cargoArtifacts;
          buildInputs = [ pkgs.sqlite.dev ];
        };
        typhon-webapp = let
          rust-wasm = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "wasm32-unknown-unknown" ];
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain rust-wasm;
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
        common-devShell-packages = [ pkgs.rustfmt ];
      in {
        packages = {
          inherit typhon typhon-webapp;
          default = typhon;
        };
        devShells = {
          default = pkgs.mkShell {
            name = "typhon-shell";
            packages = [ pkgs.diesel-cli pkgs.sqlite pkgs.pkg-config ]
              ++ common-devShell-packages;
            inputsFrom = [ typhon ];
            DATABASE_URL = "sqlite:typhon.sqlite";
          };
          typhon-webapp = pkgs.mkShell {
            name = "typhon-webapp-shell";
            packages = [ pkgs.trunk pkgs.nodePackages.sass ]
              ++ common-devShell-packages;
            inputsFrom = [ typhon-webapp ];
          };
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
