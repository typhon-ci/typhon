{
  sources ? import ./sources.nix,
  systems ? import ./systems.nix,
}: let
  lib = import ./lib {inherit sources systems;};
in {
  inherit lib;

  checks = lib.eachSystem (system: import ./checks {inherit sources system;});

  devShells = lib.eachSystem (system: import ./devshells.nix {inherit sources system;});

  packages = lib.eachSystem (system: import ./packages {inherit sources system;});

  nixosModules.default = import ./nixos/typhon.nix {inherit sources;};

  typhonJobs = import ./jobs.nix {inherit sources;};
}
