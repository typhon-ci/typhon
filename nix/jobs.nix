{sources ? import ./sources.nix}: let
  system = "x86_64-linux";
in {
  ${system} = {
    build = import ./packages/typhon.nix {inherit sources system;};
    formatted = import ./checks/formatted.nix {inherit sources system;};
    nixos = import ./checks/nixos.nix {inherit sources system;};
  };
}
