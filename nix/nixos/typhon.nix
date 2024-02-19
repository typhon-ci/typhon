{inputs ? import ../inputs.nix}: {
  config,
  lib,
  pkgs,
  ...
}: let
  inherit
    (lib)
    mkEnableOption
    mkIf
    mkOption
    types
    ;

  cfg = config.services.typhon;

  gcrootsDir = "/nix/var/nix/gcroots/typhon";
in {
  options.services.typhon = {
    enable = mkEnableOption "typhon";
    package = mkOption {
      type = types.package;
      description = "Which package to use for the Typhon instance";
      default = import ../packages/typhon.nix {
        inherit inputs;
        inherit (config.nixpkgs) system;
      };
    };
    home = mkOption {
      type = types.str;
      default = "/var/lib/typhon";
      description = "Home directory for the Typhon instance";
    };
    hashedPassword = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "The Argon2id hash of the admin password";
    };
    hashedPasswordFile = mkOption {
      type = types.nullOr types.str;
      default =
        if cfg.hashedPassword == null
        then null
        else builtins.toString (pkgs.writeText "typhon-password" cfg.hashedPassword);
      description = "Path to a file containing the Argon2id hash of the admin password";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.hashedPasswordFile != null || cfg.hashedPassword != null;
        message = "`hashedPasswordFile` or `hashedPassword` must be set";
      }
    ];

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
        ExecStart = pkgs.writeShellScript "typhon-init" ''
          [ -e ${gcrootsDir} ] || mkdir ${gcrootsDir}
          chown typhon:typhon ${gcrootsDir}
        '';
        RemainAfterExit = true;
        Type = "oneshot";
      };
    };

    systemd.services.typhon = {
      description = "Typhon service";
      wantedBy = ["multi-user.target"];
      path = [pkgs.nix pkgs.git pkgs.bubblewrap pkgs.openssh];
      serviceConfig = {
        ExecStart = pkgs.writeShellScript "typhon-start" ''
          cd ${cfg.home}
          DATABASE_URL="typhon.sqlite" ${cfg.package}/bin/typhon -p "$(cat ${cfg.hashedPasswordFile})" -v
        '';
        Type = "simple";
        User = "typhon";
        Group = "typhon";
      };
      requires = ["typhon-init.service"];
      after = ["typhon-init.service"];
    };
  };
}
