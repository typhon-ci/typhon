{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit sources system;},
  typhon ? import ../nixos/typhon.nix {inherit sources;},
}:
pkgs.testers.nixosTest ({pkgs, ...}: {
  name = "typhon-test";

  nodes = {
    typhon = {...}: {
      nixpkgs.system = system;
      nix.settings.experimental-features = ["nix-command" "flakes"];
      imports = [typhon];
      services.typhon = {
        enable = true;
        hashedPassword = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        domain = "127.0.0.1";
        webroot = "/typhon";
        https = false;
      };
    };
  };

  testScript = {nodes, ...}: let
    curl = "${pkgs.curl}/bin/curl -sf -H 'token: hello'";
    url = "http://127.0.0.1/typhon/api";
    flake = "path:${../../tests/empty}";
  in ''
    typhon.start()
    typhon.wait_for_unit("default.target")

    with subtest("Wait for Typhon"):
        typhon.wait_for_unit("typhon.service")

    with subtest("Wait for nginx"):
        typhon.wait_for_unit("nginx.service")

    with subtest("Create project"):
        typhon.succeed("${curl} -X POST --json '{\"url\":\"${flake}\",\"flake\":true}' ${url}/projects/test/create")

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
})
