utils: lib: {
  steps =
    actions:
    let
      n = builtins.length actions;
    in
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        script =
          let
            aux =
              i:
              { name, value }:
              ''
                >&2 echo "### Step ${builtins.toString i}/${builtins.toString n}: ${name}"
                echo "$stdin" | ${value.${system}}/bin/action > /dev/null
              '';
          in
          ''
            stdin=$(cat)

          ''
          + utils.nixpkgsLib.foldr (x: y: x + y) "" (utils.nixpkgsLib.imap1 aux actions)
          + ''
            echo "null"
          '';
      }
    );
}
