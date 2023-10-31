{
  sources ? import ./sources.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ./nixpkgs.nix {inherit sources system;},
}: rec {
  rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
    targets = ["wasm32-unknown-unknown"];
  };
  craneLib = sources.crane.lib.${system}.overrideToolchain rustToolchain;
}
