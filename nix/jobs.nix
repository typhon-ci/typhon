{sources ? import ./sources.nix}: let
  system = "x86_64-linux";
in {
  ${system} = {
    typhon = import ./packages/server.nix {inherit sources system;};
    typhon-webapp = import ./packages/webapp.nix {inherit sources system;};
    formatted = import ./checks/formatted.nix {inherit sources system;};
    nixos = import ./checks/nixos.nix {inherit sources system;};
  };
}
