{
  webroot ? "",
  baseurl ? "127.0.0.1:8000/api",
  https ? false,
  binaryen,
  callPackage,
  craneLib,
  lib,
  nodePackages,
  stdenv,
  trunk,
  wasm-bindgen-cli,
  writeTextFile,
}: let
  src = lib.sourceByRegex ./.. [
    "Cargo.toml"
    "Cargo.lock"
    "typhon.*"
    "typhon-types.*"
    "typhon-webapp.*"
  ];

  cargoToml = ../typhon-webapp/Cargo.toml;

  RUSTFLAGS = "--cfg=web_sys_unstable_apis";

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src cargoToml RUSTFLAGS;
    cargoExtraArgs = "-p typhon-webapp --target wasm32-unknown-unknown";
    doCheck = false;
  };

  nodeDependencies =
    (callPackage ../typhon-webapp/npm-nix {}).nodeDependencies;

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
    nativeBuildInputs = [binaryen nodePackages.sass trunk wasm-bindgen-cli];
    doCheck = false;
  };

  settings = writeTextFile {
    name = "settings.json";
    text = builtins.toJSON {inherit baseurl https;};
  };

  tarball = stdenv.mkDerivation {
    name = "source.tar.gz";
    src = ./..;
    buildPhase = ''
      tar -czf $out \
        --sort=name \
        --transform 's/^/typhon\//' \
        .
    '';
  };
in
  stdenv.mkDerivation {
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
  }
