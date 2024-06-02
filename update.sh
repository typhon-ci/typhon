#! /usr/bin/env nix-shell
#! nix-shell -i bash -p alejandra cargo cargo-edit node2nix nodejs npm-check-updates

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
node2nix -i "$NODE_DIR/package.json" -l "$NODE_DIR/package-lock.json" -o "$NODE2NIX_DIR/node-packages.nix" -c "$NODE2NIX_DIR/default.nix" -e "$NODE2NIX_DIR/node-env.nix"
alejandra "$NODE2NIX_DIR"
