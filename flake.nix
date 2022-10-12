{
  description = "Typhon";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, flake-utils, nixpkgs, crane }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.lib.${system};
        typhon = craneLib.buildPackage {
          name = "typhon";
          buildInputs = [ pkgs.sqlite.dev ];
          src = craneLib.cleanCargoSource ./.;
        };
        typhonShell = pkgs.mkShell {
          name = "typhon-shell";
          packages = [ pkgs.diesel-cli pkgs.sqlite ];
          DATABASE_URL = "sqlite:typhon.sqlite";
        };
      in {
        packages = {
          inherit typhon;
          default = typhon;
        };
        devShells.default = typhonShell;
      });
}
