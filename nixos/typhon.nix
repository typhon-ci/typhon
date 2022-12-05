typhon:
{ config, lib, pkgs, ... }:

let
  typhonPackages = typhon.packages.${config.nixpkgs.system};
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.typhon;
  index = pkgs.writeTextFile {
    name = "index.html";
    text = ''
      <!DOCTYPE html>
      <html>
        <head>
          <meta content="text/html;charset=utf-8" http-equiv="Content-Type"/>
        </head>
        <body>
          <div id="app"></div>
          <script type="module">
            import init, { app } from '${cfg.webroot}/typhon_webapp.js';

            async function run() {
              await init();
              app({
                "client_webroot": "${cfg.webroot}",
                "server_domain": "${cfg.domain}",
                "server_webroot": "${cfg.webroot}",
                "server_https": ${if cfg.https then "true" else "false"}
              });
            }

            run();
          </script>
        </body>
      </html>
    '';
  };
  webapp-root = pkgs.stdenv.mkDerivation {
    name = "typhon-webapp";
    phases = [ "installPhase" ];
    installPhase = ''
      mkdir -p $out/${cfg.webroot}
      cp ${cfg.webapp}/* $out/${cfg.webroot}
      cp ${index} $out/${cfg.webroot}/index.html
    '';
  };
  execstart = pkgs.writeShellScript "typhon-execstart" ''
    cd ${cfg.home}
    DATABASE_URL="sqlite:typhon.sqlite" ${cfg.package}/bin/typhon -p ${cfg.hashedPassword} -w ${cfg.webroot}
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
      default = typhonPackages.typhon-webapp;
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
    programs.firejail.enable = true;

    users.users.typhon = {
      home = cfg.home;
      group = "typhon";
      createHome = true;
      isSystemUser = true;
    };
    users.groups.typhon = { };

    systemd.services.typhon = {
      description = "Typhon service";
      wantedBy = [ "multi-user.target" ];
      path = [ pkgs.nix pkgs.git ];
      serviceConfig = {
        ExecStart = execstart;
        Type = "simple";
        User = "typhon";
        Group = "typhon";
      };
    };

    services.nginx = {
      enable = true;
      virtualHosts.${cfg.domain} = {
        locations."${cfg.webroot}/api" = {
          proxyPass = "http://127.0.0.1:8000";
          recommendedProxySettings = true;
          proxyWebsockets = true;
        };
        locations."${if cfg.webroot == "" then "/" else cfg.webroot}" = {
          root = webapp-root;
          extraConfig = ''
            error_page 404 =200 ${cfg.webroot}/index.html;
          '';
        };
      };
    };
  };

}
