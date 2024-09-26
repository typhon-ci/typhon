utils: _: rec {
  systems = utils.systems;
  eachSystem = utils.nixpkgsLib.genAttrs systems;
}
