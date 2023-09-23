inputs @ {nixpkgs, ...}: let
  self = {
    inherit inputs;

    lib = nixpkgs.lib;

    pkgs = nixpkgs.legacyPackages;

    importer = scope:
      nixpkgs.lib.foldr
      (file: fn: lib:
        nixpkgs.lib.recursiveUpdate
        {${scope} = import file self lib;}
        (fn lib))
      (_: {});
  };
in
  self
