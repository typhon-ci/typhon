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
        rust-analyzer
        sqlite
        ;
    };
    CURRENT_SYSTEM = system;
    RUSTC_BOOTSTRAP = 1;
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
