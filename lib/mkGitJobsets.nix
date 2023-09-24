utils: {lib}: let
  inherit
    (lib)
    eachSystem
    ;
in {
  mkGitJobsets = url:
    eachSystem (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = [
          pkgs.git
          pkgs.gnused
          pkgs.jq
        ];
        text = ''
          heads=$(git ls-remote --heads ${url} | sed 's/.*refs\/heads\/\(.*\)/\1/')
          cmd=""
          for head in $heads
          do
            cmd="$cmd . += {\"$head\": { \"flake\": \"git+${url}?ref=$head\" } } |"
          done
          array=$(echo "{}" | jq "$cmd .")
          echo "$array"
        '';
      });
}
