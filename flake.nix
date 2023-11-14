{
  description = "Typhon";

  inputs = {
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    flake-schemas.url = "github:determinatesystems/flake-schemas";

    flake-utils.url = "flake-utils";

    nixpkgs.url = "nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs: import ./nix/outputs.nix {inherit inputs;};
}
