_: lib: rec {
  mkDummyAction =
    {
      output ? "null",
    }:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [ pkgs.jq ];
        script = ''
          cat | jq -r '.input' >&2
          echo '${output}'
        '';
      }
    );
}
