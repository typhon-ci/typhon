{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit sources system;},
  rust ? import ../rust.nix {inherit sources system;},
}: let
  inherit (pkgs) lib;

  inherit (rust) craneLib;

  src = lib.sourceByRegex ../.. [
    "Cargo.toml"
    "Cargo.lock"
    "typhon.*"
  ];

  cargoToml = ../../typhon-api-client-test/Cargo.toml;

  cargoExtraArgs = "-p typhon-api-client-test";

  nativeBuildInputs = [
    pkgs.openssl
    pkgs.pkg-config
    pkgs.zlib
    pkgs.curl
  ];

  typhon-api-client-test = craneLib.buildPackage {
    inherit
      src
      cargoToml
      cargoExtraArgs
      nativeBuildInputs
      ;
  };

  typhon = import ../packages/server.nix {inherit sources system;};
in
  pkgs.stdenv.mkDerivation {
    name = "test-typhon-api";
    phases = ["configurePhase" "installPhase"];
    DATABASE_URL = "/tmp/typhon.sqlite";
    configurePhase = ''
      export HOME=$(mktemp -d)
      mkdir -p ~/.config/nix
      echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
    '';
    installPhase = ''
      # start Typhon server
      typhon -p $(echo -n password | sha256sum | head -c 64) -w "" &
      sleep 1

      # run the test client
      PROJECT_DECL="path:${../../tests/empty}" typhon-api-client-test

      # kill the server and creates $out
      kill %1 && touch $out
    '';
    nativeBuildInputs = builtins.attrValues {
      inherit
        typhon
        typhon-api-client-test
        ;
      inherit
        (pkgs)
        coreutils
        bubblewrap
        diesel-cli
        pkg-config
        sqlite
        nix
        ;
    };
  }
