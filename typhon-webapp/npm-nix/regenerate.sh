nix run 'nixpkgs#nodePackages.node2nix' -- --input ../package.json -l ../package-lock.json
nix run 'nixpkgs#alejandra' -- .
