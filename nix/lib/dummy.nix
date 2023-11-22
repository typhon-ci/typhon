utils: lib: rec {
  mkDummyAction = {output ? "null"}:
    lib.mkActionScript {
      mkPath = pkgs: [pkgs.jq];
      script = ''
        cat | jq '.input' -r >&2
        echo '${output}'
      '';
    };

  dummyStatus = mkDummyAction {};

  dummyWebhook = mkDummyAction {output = "[]";};
}
