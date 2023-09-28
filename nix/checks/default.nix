{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
}: {
  api = import ./api.nix {inherit sources system;};
  formatted = import ./formatted.nix {inherit sources system;};
  nixos = import ./nixos.nix {inherit sources system;};
}
