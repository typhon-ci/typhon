{
  inputs ? import ./inputs.nix,
  systems ? import ./systems.nix,
}: let
  self = rec {
    inherit systems;

    lib = import "${inputs.nixpkgs}/lib";

    pkgs = lib.genAttrs systems (system: import inputs.nixpkgs {inherit system;});

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
        lib.foldr
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
