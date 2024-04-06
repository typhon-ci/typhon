#!/usr/bin/env bash
nix run 'nixpkgs#nodePackages.node2nix' -- --input ../../typhon-webapp/assets/package.json
nix run 'nixpkgs#nixfmt-rfc-style' -- .
