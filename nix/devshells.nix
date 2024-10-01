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
    shellHook = ''
      export TYPHON_ROOT="$(pwd)"
      export PATH="$TYPHON_ROOT/scripts:$PATH"
      export DATABASE_URL="$TYPHON_ROOT/typhon.sqlite"
      export TYPHON_FLAKE="path:$TYPHON_ROOT/typhon-flake"
    '';
  };

  doc = pkgs.mkShell {
    name = "typhon-doc-devshell";
    packages = [ pkgs.mdbook ];
  };
}
