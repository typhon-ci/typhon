{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ./nixpkgs.nix {inherit inputs system;},
  rust ? import ./rust.nix {inherit inputs system;},
}: let
  env = ''
    export HASHED_PASSWORD=$(echo -n "password" | sha256sum | head -c 64)
    export TIMESTAMP="sec"
    export VERBOSE=3
  '';
  build = pkgs.writeShellScriptBin "build" "cargo leptos build";
  serve = pkgs.writeShellScriptBin "serve" "${env}cargo leptos serve";
  watch = pkgs.writeShellScriptBin "watch" "${env}cargo leptos watch";
in {
  default = pkgs.mkShell {
    name = "typhon-devshell";
    packages = builtins.attrValues {
      inherit (pkgs) nix;
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
      inherit build serve watch;
    };
    DATABASE_URL = "typhon.sqlite";
    TYPHON_FLAKE = ../typhon-flake;
  };

  doc = pkgs.mkShell {
    name = "typhon-doc-devshell";
    packages = [pkgs.mdbook];
  };
}
