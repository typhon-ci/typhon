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
                [[ "$jobset" =~ ${jobset} && "$system" =~ ${system} && "$job" =~ ${job} ]] && { echo "$stdin" | ${action.${system_}}/bin/action; exit $?; } || true
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
          + utils.nixpkgsLib.foldr (x: y: x + y) "" (builtins.map aux branches)
          + ''
            exit 1
          '';
      }
    );
}
