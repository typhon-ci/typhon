{
  sources ? import ../sources.nix,
  systems ? import ../systems.nix,
}: let
  utils = import ./utils.nix {inherit sources systems;};
  self =
    utils.importer null [
      ./dummy.nix
      ./github
      ./mkGitJobsets.nix
      ./mkProject.nix
      ./mkSimpleJobsets.nix
      ./mkSimpleProject.nix
      ./schemas.nix
      ./systems.nix
    ]
    self;
in
  self
