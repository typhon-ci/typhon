inputs: let
  utils = import ./utils.nix inputs;
  x =
    utils.importer "lib" [
      ./github
      ./mkAction.nix
      ./mkGitJobsets.nix
      ./mkProject.nix
      ./systems.nix
    ]
    x;
in
  x.lib
