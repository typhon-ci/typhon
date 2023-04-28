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
        typhon = let
          cargoToml = ./typhon/Cargo.toml;
          cargoArtifacts = craneLib.buildDepsOnly { inherit src cargoToml; };
        in craneLib.buildPackage {
          inherit src cargoToml cargoArtifacts;
          buildInputs = [ pkgs.sqlite.dev ];
        };
        typhon-webapp = let
          cargoToml = ./typhon-webapp/Cargo.toml;
          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src cargoToml;
            cargoExtraArgs = "-p typhon-webapp --target wasm32-unknown-unknown";
            CARGO_PROFILE = "typhon-webapp";
            doCheck = false;
          };
          nodeDependencies =
            (pkgs.callPackage ./typhon-webapp/npm-nix { }).nodeDependencies;
          webapp = craneLib.mkCargoDerivation {
            inherit src cargoToml cargoArtifacts;
            buildPhaseCargoCommand = ''
              ln -s ${nodeDependencies}/lib/node_modules typhon-webapp/node_modules
              # See #351 on Trunk
              echo "tools.wasm_bindgen = \"$(wasm-bindgen --version | cut -d' ' -f2)\"" >> Trunk.toml
              echo "build.public_url = \"WEBROOT\"" >> Trunk.toml
              trunk build --release typhon-webapp/index.html
            '';
            installPhase = "cp -r typhon-webapp/dist $out";
            nativeBuildInputs = with pkgs; [
              trunk
              wasm-bindgen-cli
              binaryen
              pkgs.nodePackages.sass
            ];
          };
        in pkgs.callPackage ({ stdenv, writeTextFile, webroot ? ""
          , baseurl ? "127.0.0.1:8000/api", https ? false }:
          let
            settings = writeTextFile {
              name = "settings.json";
              text = builtins.toJSON { inherit baseurl https; };
            };
          in stdenv.mkDerivation {
            name = "typhon-webapp";
            src = webapp;
            buildPhase = ''
              substituteInPlace ./index.html --replace "WEBROOT" "${webroot}/"
              cp "${settings}" settings.json
            '';
            installPhase = ''
              mkdir -p $out
              mv * $out
            '';
          }) { };
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
            pkgs.nodejs # npm

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
