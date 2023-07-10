{
  craneLib,
  lib,
  openssl,
  pkg-config,
}: let
  src = lib.sourceByRegex ./.. [
    "Cargo.toml"
    "Cargo.lock"
    "typhon.*"
    "typhon-types.*"
  ];

  cargoToml = ../typhon/api-client-test/Cargo.toml;

  cargoExtraArgs = "-p typhon-api-client-test";

  nativeBuildInputs = [openssl pkg-config];

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src cargoToml cargoExtraArgs nativeBuildInputs;
  };
in
  craneLib.buildPackage {
    inherit src cargoToml cargoArtifacts cargoExtraArgs nativeBuildInputs;
  }
