utils: lib: rec {
  mkDummyAction = {output ? "null"}:
    lib.builders.mkActionScript {
      mkPath = system: [utils.pkgs.${system}.jq];
      mkScript = system: ''
        cat | jq -r '.input' >&2
        echo '${output}'
      '';
    };
}
