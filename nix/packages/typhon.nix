{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit inputs system;},
  rust ? import ../rust.nix {inherit inputs system;},
}: let
  inherit (rust) craneLib;

  cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);

  args = {
    pname = "typhon";
    inherit (cargoToml.workspace.package) version;
    src = pkgs.lib.sourceByRegex ../.. [
      "Cargo.toml"
      "Cargo.lock"
      "typhon.*"
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly args;

  npm =
    import ../../typhon-webapp/assets/npm-nix
    {
      inherit system pkgs;
      nodejs = pkgs.nodejs;
    };
in
  craneLib.buildPackage (args
    // {
      inherit cargoArtifacts;
      nativeBuildInputs = [
        pkgs.cargo-leptos
        pkgs.sqlite.dev
        pkgs.binaryen
        pkgs.makeWrapper
      ];
      buildPhaseCargoCommand = "
        ln -s ${npm.nodeDependencies}/node_modules typhon-webapp/assets/node_modules
        cargo leptos build --release -vvv
      ";
      installPhaseCommand = ''
        mkdir -p $out/bin
        cp target/release/typhon $out/bin/
        cp -r target/site $out/bin/
        wrapProgram $out/bin/typhon --set LEPTOS_SITE_ROOT $out/bin/site
      '';
      TYPHON_FLAKE = ../../typhon-flake;
      doNotLinkInheritedArtifacts = true;
    })
