{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
}: rec {
  default = typhon;
  typhon = import ./typhon.nix {inherit inputs system;};
  typhon-doc = import ./doc.nix {inherit inputs system;};
}
