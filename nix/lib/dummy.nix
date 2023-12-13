utils: lib: rec {
  mkDummyAction = {output ? "null"}:
    lib.mkActionScript {
      mkPath = system: [utils.pkgs.${system}.jq];
      mkScript = system: ''
        cat | jq '.input' -r >&2
        echo '${output}'
      '';
    };

  dummyStatus = mkDummyAction {};

  dummyWebhook = mkDummyAction {output = "[]";};
}
