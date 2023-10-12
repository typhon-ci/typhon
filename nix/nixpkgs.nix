{
  sources ? import ./sources.nix,
  system ? builtins.currentSystem or "unknown-system",
}:
import sources.nixpkgs {
  inherit system;
  overlays = [
    (import sources.rust-overlay)
  ];
}
