utils: lib: {
  mkSimpleJobsets = {
    url,
    flake ? true,
    refs ? {},
  }: let
    jobsets = utils.lib.genAttrs refs (
      ref: {
        url = builtins.flakeRefToString (
          (builtins.parseFlakeRef url) // {inherit ref;}
        );
        inherit flake;
      }
    );
  in
    lib.mkDummyAction {output = builtins.toJSON jobsets;};
}
