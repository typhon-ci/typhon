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

  cargoToml = ../../typhon-webapp/Cargo.toml;

  RUSTFLAGS = "--cfg=web_sys_unstable_apis";

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src cargoToml RUSTFLAGS;
    cargoExtraArgs = "-p typhon-webapp --target wasm32-unknown-unknown";
    doCheck = false;
  };

  nodeDependencies =
    (pkgs.callPackage ../../typhon-webapp/npm-nix {}).nodeDependencies;

  inherit
    (pkgs)
    nodePackages
    binaryen
    trunk
    wasm-bindgen-cli
    ;
in
  craneLib.buildPackage {
    inherit
      src
      cargoToml
      cargoArtifacts
      RUSTFLAGS
      ;
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
    nativeBuildInputs = [
      binaryen
      nodePackages.sass
      trunk
      wasm-bindgen-cli
    ];
    doCheck = false;
  }
