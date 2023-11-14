{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ./nixpkgs.nix {inherit inputs system;},
  rust ? import ./rust.nix {inherit inputs system;},
}: {
  default = pkgs.mkShell {
    name = "typhon-devshell";
    packages = builtins.attrValues {
      inherit (pkgs.nixVersions) nix_2_18;
      inherit (rust) rustToolchain;
      inherit
        (pkgs)
        bubblewrap
        cargo-leptos
        diesel-cli
        leptosfmt
        pkg-config
        rust-analyzer
        rustfmt
        sqlite
        ;
    };
    DATABASE_URL = "sqlite:typhon.sqlite";
    TYPHON_FLAKE = ../typhon-flake;
  };

  doc = pkgs.mkShell {
    name = "typhon-doc-devshell";
    packages = [pkgs.mdbook];
  };
}
