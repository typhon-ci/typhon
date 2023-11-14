{
  inputs ? import ./inputs.nix,
  systems ? import ./systems.nix,
}: let
  lib = import ./lib {inherit inputs systems;};
in {
  inherit lib;

  checks = lib.eachSystem (system: import ./checks {inherit inputs system;});

  devShells = lib.eachSystem (system: import ./devshells.nix {inherit inputs system;});

  packages = lib.eachSystem (system: import ./packages {inherit inputs system;});

  nixosModules.default = import ./nixos/typhon.nix {inherit inputs;};

  typhonJobs = import ./jobs.nix {inherit inputs;};

  schemas = import ./schemas.nix {inherit inputs systems;};
}
