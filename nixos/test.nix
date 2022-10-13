{ system, nixpkgs, typhon }:

import "${nixpkgs}/nixos/tests/make-test-python.nix" ({ pkgs, lib, ... }: {
  name = "typhon-test";

  nodes = {
    typhon = { ... }: {
      nix.settings.experimental-features = [ "nix-command" "flakes" ];
      imports = [ typhon.nixosModules.default ];
      services.typhon = {
        enable = true;
        hashedPassword =
          "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
      };
    };
  };

  testScript = { nodes, ... }:
    let url = "http://127.0.0.1:8000";
    in ''
      typhon.start()
      typhon.wait_for_unit("default.target")

      with subtest("Wait for Typhon"):
          typhon.wait_for_unit("typhon.service")

      with subtest("Create project"):
          typhon.succeed("curl -f -H \"password: hello\" -X POST ${url}/api/create_project/test")
    '';

}) { inherit system; }
