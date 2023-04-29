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
        typhon-api-client-test = let
          cargoToml = ./typhon/api-client-test/Cargo.toml;
          cargoExtraArgs = "-p typhon-api-client-test";
          nativeBuildInputs = [ pkgs.openssl pkgs.pkg-config ];
          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src cargoToml cargoExtraArgs nativeBuildInputs;
          };
        in craneLib.buildPackage {
          inherit src cargoToml cargoArtifacts cargoExtraArgs nativeBuildInputs;
        };
        typhon-webapp = let
          cargoToml = ./typhon-webapp/Cargo.toml;
          RUSTFLAGS = "--cfg=web_sys_unstable_apis";
          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src cargoToml RUSTFLAGS;
            cargoExtraArgs = "-p typhon-webapp --target wasm32-unknown-unknown";
            doCheck = false;
          };
          nodeDependencies =
            (pkgs.callPackage ./typhon-webapp/npm-nix { }).nodeDependencies;
          webapp = craneLib.buildPackage {
            inherit src cargoToml cargoArtifacts RUSTFLAGS;
            buildPhaseCargoCommand = ''
              ln -s ${nodeDependencies}/lib/node_modules typhon-webapp/node_modules
              # See #351 on Trunk
              echo "tools.wasm_bindgen = \"$(wasm-bindgen --version | cut -d' ' -f2)\"" >> Trunk.toml
              echo "build.public_url = \"WEBROOT\"" >> Trunk.toml
              trunk build --release typhon-webapp/index.html
            '';
            # we only need to remove references on *.wasm files
            doNotRemoveReferencesToVendorDir = true;
            installPhaseCommand = ''
              cp -r typhon-webapp/dist $out
              find "$out/" -name "*.wasm" -print0 | \
                while read -d $'\0' file; do
                  removeReferencesToVendoredSources $file
                done
            '';
            nativeBuildInputs = with pkgs; [
              trunk
              wasm-bindgen-cli
              binaryen
              nodePackages.sass
            ];
            doCheck = false;
          };
        in pkgs.callPackage ({ stdenv, writeTextFile, webroot ? ""
          , baseurl ? "127.0.0.1:8000/api", https ? false }:
          let
            settings = writeTextFile {
              name = "settings.json";
              text = builtins.toJSON { inherit baseurl https; };
            };
            tarball = stdenv.mkDerivation {
              name = "source.tar.gz";
              src = ./.;
              buildPhase = ''
                tar -czf $out \
                  --sort=name \
                  --transform 's/^/typhon\//' \
                  .
              '';
            };
          in stdenv.mkDerivation {
            name = "typhon-webapp";
            src = webapp;
            buildPhase = ''
              substituteInPlace ./index.html --replace "WEBROOT" "${webroot}/"
              cp ${settings} settings.json
              cp ${tarball} source.tar.gz
            '';
            installPhase = ''
              mkdir -p $out${webroot}
              mv * $out${webroot}
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
          inherit typhon typhon-webapp typhon-doc typhon-api-client-test;
          default = typhon;
        };
        devShells.default = pkgs.mkShell {
          name = "typhon-shell";
          packages = [
            # Rust
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.openssl
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
        checks = {
          api = pkgs.stdenv.mkDerivation {
            name = "Test API";
            phases = ["configurePhase" "installPhase"];
            DATABASE_URL = "/tmp/typhon.sqlite";
            configurePhase = ''
              export HOME=$(mktemp -d)
              mkdir -p ~/.config/nix
              echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
            '';
            installPhase = ''
              # start Typhon server
              typhon -p $(echo -n password | sha256sum | head -c 64) -j null -w "" &
              sleep 1

              # run the test client
              PROJECT_DECL="path:${./tests/empty}" typhon-api-client-test

              # kill the server and creates $out
              kill %1 && touch $out
            '';
            nativeBuildInputs = [
              typhon-api-client-test
              typhon
              pkgs.coreutils
              pkgs.bubblewrap
              pkgs.diesel-cli
              pkgs.pkg-config
              pkgs.sqlite
              pkgs.nix
            ];
          };
          nixos = import ./nixos/test.nix {
            inherit system nixpkgs;
            typhon = self;
          };
        };
        actions = import ./actions { inherit pkgs; };
      }) // {
        nixosModules.default = import ./nixos/typhon.nix self;
      };
}
