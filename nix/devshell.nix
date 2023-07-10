{
  bubblewrap,
  diesel-cli,
  mdbook,
  mkShell,
  nix,
  nodePackages,
  nodejs,
  openssl,
  pkg-config,
  rust-analyzer,
  rustToolchain,
  rustfmt,
  sqlite,
  trunk,
}:
mkShell {
  name = "typhon-devshell";
  packages = [
    # Rust
    rustToolchain
    rustfmt
    rust-analyzer
    openssl

    # Typhon server
    bubblewrap
    diesel-cli
    pkg-config
    sqlite
    nix

    # Typhon webapp
    nodePackages.sass
    trunk
    nodejs # npm

    # Documentation
    mdbook
  ];
  DATABASE_URL = "sqlite:typhon.sqlite";
}
