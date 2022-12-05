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
        domain = "127.0.0.1";
        webroot = "/typhon";
        https = false;
      };
    };
  };

  testScript = { nodes, ... }:
    let
      git = "${pkgs.git}/bin/git";
      curl = "${pkgs.curl}/bin/curl -sf -H 'password: hello'";
      url = "http://127.0.0.1/typhon/api";
      flake = ../tests/empty/flake.nix;
      path = "/tmp/test";
      createRepo = pkgs.writeShellScript "create-repository" ''
        mkdir -p ${path}
        cd ${path}
        cp ${flake} ./flake.nix

        ${git} init
        ${git} config --local user.name "John Doe"
        ${git} config --local user.email johndoe@example.com
        ${git} add flake.nix
        ${git} commit -m "initial commit"
      '';
    in ''
      typhon.start()
      typhon.wait_for_unit("default.target")

      with subtest("Wait for Typhon"):
          typhon.wait_for_unit("typhon.service")

      with subtest("Wait for nginx"):
          typhon.wait_for_unit("nginx.service")

      with subtest("Create repository"):
          typhon.succeed("${createRepo}")

      with subtest("Check webapp"):
          typhon.succeed("${pkgs.curl}/bin/curl -f http://127.0.0.1/typhon")

      with subtest("Create project"):
          typhon.succeed("${curl} -X POST ${url}/projects/test/create")

      with subtest("Set project declaration"):
          typhon.succeed("${curl} -X POST --json \'\"git+file://${path}\"\' ${url}/projects/test/set_decl")

      with subtest("Refresh project"):
          typhon.succeed("${curl} -X POST ${url}/projects/test/refresh")

      with subtest("Update jobsets"):
          typhon.succeed("${curl} -X POST ${url}/projects/test/update_jobsets")

      with subtest("Evaluate jobset"):
          typhon.succeed("${curl} -X POST ${url}/projects/test/jobsets/main/evaluate")

      with subtest("Get evaluation info"):
          typhon.succeed("${curl} ${url}/projects/test/jobsets/main/evaluations/1")

      with subtest("Query non-existing evaluation"):
          typhon.fail("${curl} ${url}/projects/test/jobsets/main/evaluations/2")
    '';

}) { inherit system; }
