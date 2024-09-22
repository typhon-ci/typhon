{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import inputs.nixpkgs { inherit system; },
  craneLib ? inputs.crane.mkLib pkgs,
}:
{
  default = craneLib.devShell {
    name = "typhon-devshell";
    packages = builtins.attrValues {
      inherit (pkgs.llvmPackages) bintools; # lld
      inherit (pkgs)
        bubblewrap
        cargo-binutils
        cargo-leptos
        diesel-cli
        leptosfmt
        libargon2
        nix
        nixfmt-rfc-style
        nodejs # npm
        pkg-config
        postgresql
        rust-analyzer
        ;
    };
    CURRENT_SYSTEM = system;
    RUSTC_BOOTSTRAP = 1;
    shellHook = ''
      export TYPHON_ROOT="$(pwd)"
      export PATH="$TYPHON_ROOT/scripts:$PATH"
      export TYPHON_FLAKE="path:$TYPHON_ROOT/typhon-flake"
      export PGDATA="$TYPHON_ROOT/.postgres"
      export DATABASE_URL="postgres://localhost:5432/typhon"
    '';
  };

  doc = pkgs.mkShell {
    name = "typhon-doc-devshell";
    packages = [ pkgs.mdbook ];
  };
}
