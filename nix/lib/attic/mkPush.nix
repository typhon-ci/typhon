_: lib: {
  mkPush =
    { endpoint, cache }:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [
          pkgs.jq
          pkgs.attic-client
        ];
        script = ''
          HOME=$(pwd)
          export HOME
          stdin=$(cat)
          path=$(echo "$stdin" | jq -r '.input.out')
          ATTIC_TOKEN=$(echo "$stdin" | jq -r '.secrets.attic_token')
          attic login --set-default server ${endpoint} "$ATTIC_TOKEN"
          attic push server:${cache} "$path" >&2
        '';
      }
    );
}
