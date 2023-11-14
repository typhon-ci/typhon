{
  inputs ? import ./inputs.nix,
  systems ? import ./systems.nix,
  lib ? import ./lib {inherit inputs systems;},
  flake-schemas ? inputs.flake-schemas,
}: {
  inherit
    (lib.schemas)
    typhonJobs
    ;

  inherit
    (flake-schemas.schemas)
    checks
    devShells
    nixosModules
    packages
    schemas
    ;

  lib = {
    version = 1;
    doc = ''
      A library to build actions for [Typhon](https://typhon-ci.org/)
    '';
    allowIFD = false;
    inventory = output: {
      evalChecks = {};
      forSystems = lib.systems;
      what = "Typhon's library";
    };
  };
}
