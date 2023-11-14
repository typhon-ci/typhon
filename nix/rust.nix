{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ./nixpkgs.nix {inherit inputs system;},
}: rec {
  rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
    targets = ["wasm32-unknown-unknown"];
  };
  craneLib = inputs.crane.lib.${system}.overrideToolchain rustToolchain;
}
