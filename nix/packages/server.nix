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

  cargoToml = ../../typhon/Cargo.toml;

  cargoArtifacts = craneLib.buildDepsOnly {inherit src cargoToml;};
in
  craneLib.buildPackage {
    inherit
      src
      cargoToml
      cargoArtifacts
      ;
    buildInputs = [pkgs.sqlite.dev];
    TYPHON_FLAKE = ../../typhon-flake;
  }
