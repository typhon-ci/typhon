{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
}: rec {
  default = typhon;
  typhon = import ./server.nix {inherit sources system;};
  typhon-webapp = import ./webapp.nix {inherit sources system;};
  typhon-doc = import ./doc.nix {inherit sources system;};
}
