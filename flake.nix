{
  description = "Typhon";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = inputs @ {
    self,
    flake-utils,
    nixpkgs,
    crane,
    rust-overlay,
  }: let
    lib = import ./lib inputs;
  in
    flake-utils.lib.eachSystem lib.systems (system: let
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
        formatted = pkgs.callPackage ./nix/check-formatted.nix {inherit rustToolchain;};
        nixos = pkgs.callPackage ./nix/nixos/test.nix {typhon = self;};
      };
    })
    // {
      inherit lib;
      nixosModules.default = import ./nix/nixos/typhon.nix self;
    };
}
