utils: lib: let
  inherit
    (lib)
    systems
    ;
in {
  eachSystem = utils.lib.genAttrs systems;
}
