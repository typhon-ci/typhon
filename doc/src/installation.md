# Installation

## NixOS

At the moment the preferred way to install Typhon is on NixOS via the exposed
module.

### Example

Here is a sample NixOS module that deploys a Typhon instance:

```nix
{ ... }:

let typhon = builtins.getFlake "github:typhon-ci/typhon";
in {
  imports = [ typhon.nixosModules.default ];

  # enable flakes
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # enable Typhon
  services.typhon = {
    enable = true;

    # the admin password
    # $ echo -n hello | sha256sum | head -c 64
    hashedPassword =
      "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    # the domain of your instance
    domain = "typhon-ci.org";

    # the webroot at wich the instance is served
    webroot = "";

    # enable https, you must configure it manually in nginx
    https = true;
  };

  # configure https
  services.nginx.virtualHosts."typhon-ci.org" = {
    forceSSL = true;
    enableACME = true;
  };
}
```


### Options

Here is a list of options exposed by the NixOS module.

Mandatory:

- `services.typhon.enable`: a boolean to activate the Typhon instance.
- `services.typhon.hashedPassword`: a string containing the digest of the admin
  password. Use `sha256sum` to compute this value.
- `services.typhon.domain`: a string containing the domain at which the Typhon
  instance is served.
- `services.typhon.webroot`: a string containing the webroot at wich the Typhon
  instance is served. To serve at the top-level, simply use an empty string,
  otherwise use a path with no trailing slash (for example "/typhon/webroot").
- `services.typhon.https`: a boolean to set if the instance is served with
  https. At the moment this does not automatically configure nginx to use https.

Optional:

- `services.typhon.home`: a string containing the home directory of the Typhon
  instance.
- `services.typhon.package`: a derivation to override the package used for the
  Typhon instance.
- `services.typhon.webapp`: a derivation to override the package used for the
  webapp.
