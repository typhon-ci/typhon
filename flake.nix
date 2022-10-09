{
  description = "Typhon";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.follows = "crane/nixpkgs";
  };

  outputs = { self, crane, nixpkgs }:
    let
      system = "x86_64-linux";
      craneLib = crane.lib.${system};
      pkgs = import nixpkgs { inherit system; };
      typhon = craneLib.buildPackage {
        buildInputs = [ pkgs.sqlite.dev ];
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            path == toString ./Cargo.toml || path == toString ./Cargo.lock
            || pkgs.lib.hasPrefix (toString ./src) path;
        };
      };
      typhonShell = pkgs.mkShell {
        packages = [ pkgs.diesel-cli pkgs.sqlite pkgs.sqlitebrowser ];
        DATABASE_URL = "sqlite:typhon.sqlite";
      };
    in {
      packages.${system} = {
        inherit typhon;
        default = typhon;
      };
      devShells.${system}.default = typhonShell;
      nixosModules.default = import ./nixos/typhon.nix typhon;
      checks.${system}.default = import ./nixos/test.nix {
        inherit system nixpkgs;
        typhon = self;
      };
    };
}
