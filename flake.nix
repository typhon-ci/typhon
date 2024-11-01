{
  description = "Typhon";

  inputs = {
    flake-compat.url = "https://git.lix.systems/api/v1/repos/lix-project/flake-compat/archive/main.tar.gz";

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs: import ./nix/outputs.nix { inherit inputs; };
}
