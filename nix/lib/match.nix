utils: lib: {
  match = branches:
    lib.mkActionScript {
      mkScript = system_: let
        aux = {
          jobset ? ".*",
          system ? ".*",
          job ? ".*",
          action,
        }: ''
          [[ "$jobset" =~ ${jobset} && "$system" =~ ${system} && "$job" =~ ${job} ]] && { echo "$json" | ${action.${system_}}/bin/action; exit $?; } || true
        '';
      in
        ''
          json=$(cat)
          jobset=$(echo "$json" | jq '.input.jobset' -r)
          system=$(echo "$json" | jq '.input.system' -r)
          job=$(echo "$json" | jq '.input.job' -r)
        ''
        + utils.lib.foldr (x: y: x + y) "" (builtins.map aux branches)
        + ''
          exit 1
        '';
    };
}
