utils: lib: {
  mkJobsets =
    {
      url,
      flake ? true,
      refs ? { },
    }:
    let
      jobsets = utils.nixpkgsLib.genAttrs refs (ref: {
        url = builtins.flakeRefToString ((builtins.parseFlakeRef url) // { inherit ref; });
        inherit flake;
      });
    in
    lib.builders.mkDummyAction { output = builtins.toJSON jobsets; };
}
