utils: _: rec {
  systems = utils.systems;
  eachSystem = utils.lib.genAttrs systems;
}
