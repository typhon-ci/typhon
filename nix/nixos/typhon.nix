{sources ? import ../sources.nix}: {
  config,
  lib,
  pkgs,
  ...
}: let
  typhonPackages = import ../packages {
    inherit sources;
    inherit (config.nixpkgs) system;
  };

  inherit
    (lib)
    mkEnableOption
    mkIf
    mkOption
    types
    ;

  cfg = config.services.typhon;

  gcrootsDir = "/nix/var/nix/gcroots/typhon";

  init-execstart = pkgs.writeShellScript "typhon-init" ''
    [ -e ${gcrootsDir} ] || mkdir ${gcrootsDir}
    chown typhon:typhon ${gcrootsDir}
  '';

  protocol =
    if cfg.https
    then "https"
    else "http";

  typhon-execstart = pkgs.writeShellScript "typhon-execstart" ''
    cd ${cfg.home}
    DATABASE_URL="sqlite:typhon.sqlite" ${cfg.package}/bin/typhon -p ${cfg.hashedPassword} -w "${cfg.webroot}"
  '';
in {
  options.services.typhon = {
    enable = mkEnableOption "typhon";
    package = mkOption {
      type = types.package;
      description = "Which package to use for the Typhon instance";
      default = typhonPackages.typhon;
    };
    webapp = mkOption {
      type = types.package;
      description = "Which webapp to use for the Typhon instance";
      default = typhonPackages.typhon-webroot;
    };
    home = mkOption {
      type = types.str;
      default = "/var/lib/typhon";
      description = "Home directory for the Typhon instance";
    };
    hashedPassword = mkOption {
      type = types.str;
      description = "Sha256 of the admin password for the Typhon instance";
    };
    domain = mkOption {
      type = types.str;
      description = "Domain name for the Typhon instance";
    };
    webroot = mkOption {
      type = types.str;
      description = "Web root for the Typhon instance";
    };
    https = mkOption {
      type = types.bool;
      description = "Enable https for the Typhon instance";
    };
  };

  config = mkIf cfg.enable {
    users.users.typhon = {
      home = cfg.home;
      group = "typhon";
      createHome = true;
      isSystemUser = true;
    };
    users.groups.typhon = {};

    systemd.services.typhon-init = {
      description = "Typhon init";
      wantedBy = ["multi-user.target"];
      serviceConfig = {
        ExecStart = init-execstart;
        RemainAfterExit = true;
        Type = "oneshot";
      };
    };

    systemd.services.typhon = {
      description = "Typhon service";
      wantedBy = ["multi-user.target"];
      path = [pkgs.nixVersions.nix_2_18 pkgs.git pkgs.bubblewrap];
      serviceConfig = {
        ExecStart = typhon-execstart;
        Type = "simple";
        User = "typhon";
        Group = "typhon";
      };
      requires = ["typhon-init.service"];
      after = ["typhon-init.service"];
    };

    services.nginx = {
      enable = true;
      virtualHosts.${cfg.domain} = {
        locations."${cfg.webroot}/api" = {
          proxyPass = "http://127.0.0.1:8000";
          recommendedProxySettings = true;
          proxyWebsockets = true;
        };
        locations."${
          if cfg.webroot == ""
          then "/"
          else cfg.webroot
        }" = {
          root = cfg.webapp.override {
            inherit (cfg) webroot;
            api_url = "${protocol}://${cfg.domain}${cfg.webroot}/api";
          };
          tryFiles = "$uri $uri/ ${cfg.webroot}/index.html =404";
        };
      };
    };
  };
}
