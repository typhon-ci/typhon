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

    importList = scope: list:
      mkScope scope (
        lib.foldr
        (path: fn: lib:
          unionOfDisjoint
          (import path self lib)
          (fn lib))
        (_: {})
        list
      );

    importPath = scope: path:
      importList scope (
        lib.mapAttrsToList (x: _: "${path}/${x}") (
          builtins.removeAttrs (builtins.readDir path) ["default.nix"]
        )
      );
  };
in
  self
