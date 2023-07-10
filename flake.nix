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
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = {
    self,
    flake-utils,
    nixpkgs,
    crane,
    rust-overlay,
  }:
    flake-utils.lib.eachSystem ["x86_64-linux"] (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = ["wasm32-unknown-unknown"];
      };
      craneLib = crane.lib.${system}.overrideToolchain rustToolchain;

      typhon = pkgs.callPackage ./nix/server.nix {inherit craneLib;};
      typhon-webapp = pkgs.callPackage ./nix/webapp.nix {inherit craneLib;};
      typhon-doc = pkgs.callPackage ./nix/doc.nix {};
      typhon-api-client-test =
        pkgs.callPackage ./nix/api-client-test.nix {inherit craneLib;};
      typhon-devshell =
        pkgs.callPackage ./nix/devshell.nix {inherit rustToolchain;};
      typhon-actions = import ./nix/actions {inherit pkgs;};
    in {
      packages = {
        inherit typhon typhon-webapp typhon-doc typhon-api-client-test;
        default = typhon;
      };

      devShells.default = typhon-devshell;

      checks = {
        api = pkgs.callPackage ./nix/check-api.nix {
          inherit typhon typhon-api-client-test;
        };
        nixos = pkgs.callPackage ./nix/nixos/test.nix {typhon = self;};
      };

      actions = typhon-actions;
    })
    // {
      nixosModules.default = import ./nix/nixos/typhon.nix self;
    };
}
