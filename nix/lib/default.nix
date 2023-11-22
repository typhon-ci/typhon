{
  inputs ? import ../inputs.nix,
  systems ? import ../systems.nix,
}: let
  utils = import ./utils.nix {inherit inputs systems;};
  self =
    utils.importer null [
      ./dummy.nix
      ./github
      ./mkGitJobsets.nix
      ./mkActionScript.nix
      ./mkProject.nix
      ./mkSimpleJobsets.nix
      ./mkSimpleProject.nix
      ./schemas.nix
      ./systems.nix
    ]
    self;
in
  self
