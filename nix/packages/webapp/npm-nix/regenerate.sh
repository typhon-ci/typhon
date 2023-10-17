#!/usr/bin/env bash
root="../../../.."
nix run 'nixpkgs#nodePackages.node2nix' -- \
    --input "$root/typhon-webapp/package.json" \
    -l "$root/typhon-webapp/package-lock.json"
nix run 'nixpkgs#alejandra' -- .
