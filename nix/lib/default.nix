{
  inputs ? import ../inputs.nix,
  systems ? import ../systems.nix,
  utils ? import ../utils.nix {inherit inputs systems;},
}: let
  self = utils.importPath null ./. self;
in
  self
