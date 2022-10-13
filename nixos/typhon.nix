typhon:
{ config, lib, pkgs, ... }:

let
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.typhon;
  execstart = pkgs.writeShellScript "typhon-execstart" ''
    cd ${cfg.home}
    DATABASE_URL="sqlite:typhon.sqlite" ${cfg.package}/bin/typhon -p ${cfg.hashedPassword}
  '';
in {
  options.services.typhon = {
    enable = mkEnableOption "typhon";
    package = mkOption {
      type = types.package;
      description = "Which package to use for the Typhon instance";
      default = typhon.packages.${config.nixpkgs.system}.typhon;
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
  };
}
