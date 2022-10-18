{ pkgs }: rec {

  mkProject = args@{ actions ? { }, meta ? { }, ... }:
    let
      linkIfExists = attrset: key:
        if attrset ? ${key} then "ln -s ${attrset.${key}} ${key}" else "";
    in {
      inherit meta;
      actions = pkgs.stdenv.mkDerivation {
        name = "actions";
        phases = [ "installPhase" ];
        installPhase = ''
          mkdir $out
          cd $out
          ${linkIfExists actions "jobsets"}
          ${linkIfExists actions "begin"}
          ${linkIfExists actions "end"}
          ${linkIfExists args "secrets"}
        '';
      };
    };

  mkAction = { packages, script }:
    let
      path =
        pkgs.lib.foldr (a: b: "${a}/bin:${b}") "${pkgs.coreutils}/bin" packages;
    in pkgs.writeShellScript "action" ''
      export PATH=${path}
      ${script}
    '';

  gitJobsets = url:
    mkAction {
      packages = [ pkgs.git pkgs.gnused pkgs.jq ];
      script = ''
        heads=$(git ls-remote --heads ${url} | sed 's/.*refs\/heads\/\(.*\)/\1/')
        for head in $heads
        do
          cmd="$cmd . += {\"$head\": { \"flake\": \"git+${url}?ref=$head\" } } |"
        done
        array=$(echo "{}" | jq "$cmd .")
        echo $array
      '';
    };

}
