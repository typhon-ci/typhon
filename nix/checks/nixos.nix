{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import inputs.nixpkgs { inherit system; },
  typhon ? import ../nixos/typhon.nix { inherit inputs; },
}:
pkgs.testers.nixosTest (
  { pkgs, ... }:
  {
    name = "typhon-test";

    nodes = {
      typhon =
        { ... }:
        {
          nixpkgs.system = system;
          nix.settings.experimental-features = [
            "nix-command"
            "flakes"
          ];
          imports = [ typhon ];
          services.typhon = {
            enable = true;
            hashedPasswordFile = builtins.toString (
              pkgs.runCommand "password" { } ''
                echo -n "password" | ${pkgs.libargon2}/bin/argon2 "Guérande" -id -e > $out
              ''
            );
          };
          services.nginx = {
            enable = true;
            virtualHosts."example.com" = {
              locations."/" = {
                proxyPass = "http://localhost:3000";
                recommendedProxySettings = true;
              };
            };
          };
        };
    };

    testScript =
      { nodes, ... }:
      let
        curl = "${pkgs.curl}/bin/curl -sf -H 'password: password'";
        url = "http://127.0.0.1/api";
      in
      ''
        typhon.start()
        typhon.wait_for_unit("default.target")

        with subtest("Wait for Typhon"):
            typhon.wait_for_unit("typhon.service")

        with subtest("Wait for nginx"):
            typhon.wait_for_unit("nginx.service")

        with subtest("Create project"):
            typhon.succeed("${curl} --json '{\"url\":\"github:typhon-ci/typhon\",\"flake\":true}' ${url}/projects/typhon/create")

        with subtest("Get project info"):
            typhon.succeed("${curl} ${url}/projects/typhon")

        with subtest("Query non-existing evaluation"):
            typhon.fail("${curl} ${url}/projects/typhon/evaluations/1")
      '';
  }
)
