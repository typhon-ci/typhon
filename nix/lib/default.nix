{
  sources ? import ../sources.nix,
  systems ? import ../systems.nix,
}: let
  utils = import ./utils.nix {inherit sources systems;};
  self =
    utils.importer null [
      ./dummyWebhook.nix
      ./github
      ./mkGitJobsets.nix
      ./mkProject.nix
      ./schemas.nix
      ./systems.nix
    ]
    self;
in
  self
