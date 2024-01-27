# Installation

## Nix requirements

Typhon requires Nix >= 2.18 with experimental features "nix-command" and
"flakes" enabled.

## NixOS

At the moment the preferred way to install Typhon is on NixOS via the exposed
module.

### Example

Here is a sample NixOS module that deploys a Typhon instance:

```nix
{ pkgs, ... }:

let typhon = builtins.getFlake "github:typhon-ci/typhon";
in {
  imports = [ typhon.nixosModules.default ];

  # enable experimental features
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # install Nix >= 2.18 if necessary
  nix.package = pkgs.nixVersions.nix_2_18;

  # enable Typhon
  services.typhon = {
    enable = true;

    # the admin password
    # $ echo -n "password" | sha256sum | head -c 64
    hashedPassword =
      "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8";
  };

  # configure nginx
  services.nginx.virtualHosts."typhon-ci.org" = {
    enable = true;
    forceSSL = true;
    enableACME = true;
    virtualHosts."example.com" = {
      locations."/" = {
        proxyPass = "http://127.0.0.1:3000";
        recommendedProxySettings = true;
      };
    };
  };
}
```


### Options

Here is a list of options exposed by the NixOS module.

Mandatory:

- `services.typhon.enable`: a boolean to activate the Typhon instance.
- `services.typhon.hashedPassword`: a string containing the digest of the admin
  password. Use `sha256sum` to compute this value.

Optional:

- `services.typhon.home`: a string containing the home directory of the Typhon
  instance.
- `services.typhon.package`: a derivation to override the package used for the
  Typhon instance.
