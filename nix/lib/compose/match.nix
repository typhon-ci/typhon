utils: lib: {
  match =
    branches:
    lib.builders.mkActionScript (
      { pkgs, system }:
      let
        system_ = system;
      in
      {
        path = [ pkgs.jq ];
        script =
          let
            aux =
              {
                jobset ? ".*",
                system ? ".*",
                job ? ".*",
                action,
              }:
              ''
                if [[ "$jobset" =~ ${jobset} && "$system" =~ ${system} && "$job" =~ ${job} ]]
                then
                  echo "$stdin" | ${action.${system_}}/bin/action
                  exit $?
                fi
              '';
          in
          ''
            stdin=$(cat)

            input=$(echo "$stdin" | jq -r '.input | ${utils.jqJsonToBashArray}')
            declare -A input="($input)"

            jobset=''${input[jobset]}
            system=''${input[system]}
            job=''${input[job]}

          ''
          + utils.lib.foldr (x: y: x + y) "" (builtins.map aux branches)
          + ''
            exit 1
          '';
      }
    );
}
