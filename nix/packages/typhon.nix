{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix { inherit inputs system; },
  rust ? import ../rust.nix { inherit inputs system; },
}:
let
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

  nodeDependencies = (import ../npm-nix { inherit system pkgs; }).nodeDependencies;
in
craneLib.buildPackage (
  args
  // {
    inherit cargoArtifacts;
    nativeBuildInputs = [
      pkgs.cargo-leptos
      pkgs.sqlite.dev
      pkgs.binaryen
      pkgs.makeWrapper
    ];
    buildPhaseCargoCommand = "cargo leptos build --release -vvv";
    installPhaseCommand = ''
      mkdir -p $out/bin
      cp target/release/typhon $out/bin/
      cp -r target/site $out/bin/
      wrapProgram $out/bin/typhon --set LEPTOS_SITE_ROOT $out/bin/site
    '';
    CURRENT_SYSTEM = system;
    TYPHON_FLAKE = "path:${../../typhon-flake}";
    doNotLinkInheritedArtifacts = true;
    preFixup = "cp -r ${nodeDependencies}/lib/node_modules $out/bin/site";
  }
)
