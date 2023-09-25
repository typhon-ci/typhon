inputs @ {nixpkgs, ...}: let
  self = rec {
    inherit inputs;

    lib = nixpkgs.lib;

    pkgs = nixpkgs.legacyPackages;

    unionOfDisjoint = x: y:
      if builtins.intersectAttrs x y == {}
      then x // y
      else throw "unionOfDisjoint: intersection is not empty";

    mkScope = scope: fn:
      if scope == null
      then fn
      else lib: {${scope} = fn lib;};

    importer = scope: files:
      mkScope scope (
        nixpkgs.lib.foldr
        (file: fn: lib:
          unionOfDisjoint
          (import file self lib)
          (fn lib))
        (_: {})
        files
      );
  };
in
  self
