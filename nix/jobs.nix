{
  inputs ? import ./inputs.nix,
}:
let
  system = "x86_64-linux";
in
{
  build = import ./packages/typhon.nix { inherit inputs system; };
  doc = import ./packages/doc.nix { inherit inputs system; };
  formatted = import ./checks/formatted.nix { inherit inputs system; };
  nixos = import ./checks/nixos.nix { inherit inputs system; };
}
