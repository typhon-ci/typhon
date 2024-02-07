{
  inputs ? import ./inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ./nixpkgs.nix {inherit inputs system;},
  rust ? import ./rust.nix {inherit inputs system;},
}: let
  env = ''
    export PASSWORD=/dev/null
    export COOKIE_SECRET=$(seq 100 | xxd -cu -l 64 -p)
    export TIMESTAMP="sec"
    export VERBOSE=3
  '';
  build = pkgs.writeShellScriptBin "build" "cargo leptos build";
  serve = pkgs.writeShellScriptBin "serve" "${env}cargo leptos serve";
  watch = pkgs.writeShellScriptBin "watch" "${env}cargo leptos watch";
  format = pkgs.writeShellScriptBin "format" "alejandra . ; rustfmt typhon*/ ; leptosfmt typhon*/";
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
        alejandra
        sqlite
        ;
      inherit build serve watch format;
    };
    DATABASE_URL = "typhon.sqlite";
    TYPHON_FLAKE = ../typhon-flake;
  };

  doc = pkgs.mkShell {
    name = "typhon-doc-devshell";
    packages = [pkgs.mdbook];
  };
}
