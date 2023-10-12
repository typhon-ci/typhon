{
  description = "Typhon";

  inputs = {
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    flake-utils.url = "flake-utils";

    nixpkgs.url = "nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";

    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs: import ./nix/outputs.nix {sources = inputs;};
}
