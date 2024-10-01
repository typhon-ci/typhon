{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import inputs.nixpkgs { inherit inputs system; },
  craneLib ? inputs.crane.mkLib pkgs,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);

  args = {
    pname = "typhon";
    inherit (cargoToml.workspace.package) version;
    src = pkgs.lib.sourceByRegex ../.. [
      "Cargo.toml"
      "Cargo.lock"
      "typhon.*"
    ];
    RUSTC_BOOTSTRAP = 1;
  };

  nativeBuildInputs = [
    pkgs.cargo-leptos
    pkgs.cargo-binutils
    pkgs.llvmPackages.bintools
    pkgs.binaryen
  ];

  leptosToml = pkgs.writeText "leptos.toml" ''
    [[workspace.metadata.leptos]]
    name = "typhon"
    bin-package = "typhon"
    lib-package = "typhon-webapp"
    lib-features = ["hydrate"]
  '';

  cargoArtifacts = craneLib.buildDepsOnly (
    args
    // {
      extraDummyScript = ''
        chmod +w $out/Cargo.toml
        cat ${leptosToml} >> $out/Cargo.toml
        mv $out/typhon/src/bin/crane-dummy-typhon/main.rs $out/typhon/src/
      '';
      inherit nativeBuildInputs;
      buildPhaseCargoCommand = "cargo leptos build --release -vvv";
    }
  );

  nodeDependencies = (import ../npm-nix { inherit system pkgs; }).nodeDependencies;
in
craneLib.buildPackage (
  args
  // {
    inherit cargoArtifacts;
    nativeBuildInputs = nativeBuildInputs ++ [
      pkgs.sqlite.dev
      pkgs.makeWrapper
    ];
    buildPhaseCargoCommand = "cargo leptos build --release -vvv";
    installPhaseCommand = ''
      mkdir -p $out/bin
      cp target/release/typhon $out/bin/
      cp -r target/site $out/bin/
      cp -r ${nodeDependencies}/lib/node_modules $out/bin/site
      chmod +w -R $out/bin/site/node_modules
      wrapProgram $out/bin/typhon --set LEPTOS_SITE_ROOT $out/bin/site
    '';
    CURRENT_SYSTEM = system;
    TYPHON_FLAKE = "path:${../../typhon-flake}";
    doCheck = false;
    doNotLinkInheritedArtifacts = true;
  }
)
