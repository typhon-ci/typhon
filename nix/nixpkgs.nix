{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
}:
import inputs.nixpkgs {
  inherit system;
  overlays = [
    (import inputs.rust-overlay)
  ];
}
