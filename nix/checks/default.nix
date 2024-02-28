{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
}: {
  formatted = import ./formatted.nix {inherit inputs system;};
  nixos = import ./nixos.nix {inherit inputs system;};
  lib = import ./lib.nix {inherit inputs system;};
}
