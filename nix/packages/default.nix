{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
}: rec {
  default = typhon;
  typhon = import ./typhon.nix {inherit sources system;};
  typhon-doc = import ./doc.nix {inherit sources system;};
}
