{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  lib ? import ../lib { inherit inputs; },
  pkgs ? import ../nixpkgs.nix { inherit inputs system; },
  utils ? import ../utils.nix { inherit inputs; },
}:
let
  owner = "typhon";
  repo = "typhon-ci";
  typhonUrl = "https://etna.typhon-ci.org/";
  secrets = pkgs.writeText "secrets" "";
  all = {
    github =
      (lib.github.mkProject {
        inherit
          owner
          repo
          typhonUrl
          secrets
          ;
        deploy = [
          {
            name = "push to cachix";
            value = lib.cachix.mkPush { name = "typhon"; };
          }
          {
            name = "github pages";
            value = lib.github.mkPages {
              inherit owner repo;
              job = "main";
              customDomain = "typhon-ci.org";
            };
          }
        ];
      }).actions;
    gitea =
      (lib.gitea.mkProject {
        inherit
          owner
          repo
          typhonUrl
          secrets
          ;
        instance = "codeberg.org";
      }).actions;
    dummy = (lib.dummy.mkProject { url = "github:typhon-ci/typhon"; }).actions;
    git = lib.git.mkJobsets { url = "https://github.com/typhon-ci/typhon"; };
  };
in
pkgs.stdenv.mkDerivation {
  name = "check-typhon-lib";
  buildInputs = utils.lib.mapAttrsToList (_: x: x.${system}) all;
  phases = [ "installPhase" ];
  installPhase = "touch $out";
}
