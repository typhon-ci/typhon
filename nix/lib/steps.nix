utils: lib: {
  steps = actions: let
    n = builtins.length actions;
  in
    lib.mkActionScript {
      mkScript = system: let
        aux = i: {
          name,
          value,
        }: ''
          >&2 echo "### Step ${builtins.toString i}/${builtins.toString n}: ${name}"
          echo "$json" | ${value.${system}}/bin/action > /dev/null
        '';
      in
        ''
          json=$(cat)
        ''
        + utils.lib.foldr (x: y: x + y) "" (utils.lib.imap1 aux actions)
        + ''
          echo "null"
        '';
    };
}
