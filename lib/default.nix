inputs: let
  utils = import ./utils.nix inputs;
  x =
    utils.importer "lib" [
      ./eachSystem.nix
      ./github
      ./mkGitJobsets.nix
      ./mkProject.nix
      ./systems.nix
    ]
    x;
in
  x.lib
