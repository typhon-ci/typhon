{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ./nixpkgs.nix { inherit inputs system; },
  rust ? import ./rust.nix { inherit inputs system; },
}:
{
  default = pkgs.mkShell {
    name = "typhon-devshell";
    packages = builtins.attrValues {
      inherit (rust) rustToolchain;
      inherit (pkgs)
        bubblewrap
        cargo-leptos
        diesel-cli
        leptosfmt
        libargon2
        nix
        nixfmt-rfc-style
        nodejs # npm
        pkg-config
        rust-analyzer
        rustfmt
        sqlite
        ;
    };
    CURRENT_SYSTEM = system;
    DATABASE_URL = "typhon.sqlite";
    TYPHON_FLAKE = ../typhon-flake;
    shellHook = ''
      export TYPHON_ROOT="$(pwd)"
      export PATH="$TYPHON_ROOT/scripts:$PATH"
    '';
  };

  doc = pkgs.mkShell {
    name = "typhon-doc-devshell";
    packages = [ pkgs.mdbook ];
  };
}
