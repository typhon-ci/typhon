utils: {lib}: let
  inherit
    (lib)
    systems
    ;
in {
  mkAction = utils.lib.genAttrs systems;
}
