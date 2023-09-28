{
  sources ? import ../sources.nix,
  systems ? import ../systems.nix,
}: let
  utils = import ./utils.nix {inherit sources systems;};
  self =
    utils.importer null [
      ./dummyWebhook.nix
      ./eachSystem.nix
      ./github
      ./mkGitJobsets.nix
      ./mkProject.nix
      ./systems.nix
    ]
    self;
in
  self
