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
          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src cargoExtraArgs;
            doCheck = false;
          };
          wasm = craneLib.buildPackage {
            name = "typhon-webapp-wasm";
            inherit src cargoArtifacts cargoExtraArgs;
            doCheck = false;
          };
        in pkgs.stdenv.mkDerivation {
          name = "typhon-webapp";
          phases = [ "buildPhase" ];
          nativeBuildInputs = [ pkgs.wasm-bindgen-cli ];
          buildPhase = ''
            wasm-bindgen ${wasm}/lib/typhon_webapp.wasm --out-dir $out --target web
          '';
        };
        webapp-root = let
          index = pkgs.writeTextFile {
            name = "index.html";
            text = ''
              <!DOCTYPE html>
              <html>
                <head>
                  <meta content="text/html;charset=utf-8" http-equiv="Content-Type"/>
                </head>
                <body>
                  <div id="app"></div>
                  <script type="module">
                    import init, { app } from './typhon_webapp.js';

                    async function run() {
                      await init();
                      app({
                        "client_webroot": "",
                        "server_domain": "127.0.0.1:8000",
                        "server_webroot": "",
                        "server_https": false
                      });
                    }

                    run();
                  </script>
                </body>
              </html>
            '';
          };
        in pkgs.stdenv.mkDerivation {
          name = "typhon-webapp";
          phases = [ "installPhase" ];
          installPhase = ''
            mkdir -p $out
            ln -s ${typhon-webapp}/* $out
            ln -s ${index} $out/index.html
          '';
        };
        common-devShell-packages = [ pkgs.rustfmt ];
      in {
        packages = {
          inherit typhon typhon-webapp webapp-root;
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
