{
  description = "Typhon";

  inputs = {
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    flake-schemas.url = "github:determinatesystems/flake-schemas";

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs: import ./nix/outputs.nix { inherit inputs; };
}
