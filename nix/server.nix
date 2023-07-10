{
  craneLib,
  lib,
  sqlite,
}: let
  src = lib.sourceByRegex ./.. [
    "Cargo.toml"
    "Cargo.lock"
    "typhon.*"
    "typhon-types.*"
  ];

  cargoToml = ../typhon/Cargo.toml;

  cargoArtifacts = craneLib.buildDepsOnly {inherit src cargoToml;};
in
  craneLib.buildPackage {
    inherit src cargoToml cargoArtifacts;
    buildInputs = [sqlite.dev];
  }
