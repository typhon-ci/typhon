utils: lib: {
  match =
    branches:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [ pkgs.jq ];
        script =
          let
            aux =
              {
                jobset ? ".*",
                job ? ".*",
                action,
              }:
              ''
                [[ "$jobset" =~ ${jobset} && "$job" =~ ${job} ]] && { echo "$stdin" | ${action.${system}}/bin/action; exit $?; } || true
              '';
          in
          ''
            stdin=$(cat)

            input=$(echo "$stdin" | jq -r '.input | ${utils.jqJsonToBashArray}')
            declare -A input="($input)"

            jobset=''${input[jobset]}
            job=''${input[job]}

          ''
          + utils.nixpkgsLib.foldr (x: y: x + y) "" (builtins.map aux branches)
          + ''
            exit 1
          '';
      }
    );
}
