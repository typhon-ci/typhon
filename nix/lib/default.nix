{
  inputs ? import ../inputs.nix,
  systems ? import ../systems.nix,
}: let
  utils = import ./utils.nix {inherit inputs systems;};
  self =
    utils.importer null [
      ./dummy.nix
      ./github
      ./match.nix
      ./mkActionScript.nix
      ./mkGitJobsets.nix
      ./mkProject.nix
      ./mkSimpleJobsets.nix
      ./mkSimpleProject.nix
      ./schemas.nix
      ./steps.nix
      ./systems.nix
    ]
    self;
in
  self
