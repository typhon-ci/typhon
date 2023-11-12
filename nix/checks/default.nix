{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
}: {
  formatted = import ./formatted.nix {inherit sources system;};
  nixos = import ./nixos.nix {inherit sources system;};
}
