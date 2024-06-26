#! /usr/bin/env nix-shell
#! nix-shell -i bash -p cargo cargo-edit nixfmt-rfc-style node2nix nodejs npm-check-updates

NODE_DIR="typhon-webapp/assets"
NODE2NIX_DIR="nix/npm-nix"

# update Rust dependencies
cargo upgrade -i
cargo update

# update Node dependencies
ncu -u --packageFile "$NODE_DIR/package.json"
npm install --prefix "$NODE_DIR"

# update Nix dependencies
nix flake update
node2nix -18 -i "$NODE_DIR/package.json" -o "$NODE2NIX_DIR/node-packages.nix" -c "$NODE2NIX_DIR/default.nix" -e "$NODE2NIX_DIR/node-env.nix"
nixfmt "$NODE2NIX_DIR"
