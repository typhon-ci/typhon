inputs: let
  utils = import ./utils.nix inputs;
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
