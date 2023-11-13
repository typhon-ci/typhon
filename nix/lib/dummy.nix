utils: lib: rec {
  mkDummyAction = {output ? "null"}:
    lib.eachSystem (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = [pkgs.jq];
        text = ''
          cat | jq '.input' -r >&2
          echo '${output}'
        '';
      });

  dummyStatus = mkDummyAction {};

  dummyWebhook = mkDummyAction {output = "[]";};
}
