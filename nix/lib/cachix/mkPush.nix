_: lib: {
  mkPush = {name}:
    lib.builders.mkActionScript ({
      pkgs,
      system,
    }: {
      path = [
        pkgs.jq
        pkgs.cachix
      ];
      script = ''
        stdin=$(cat)
        path=$(echo "$stdin" | jq -r '.input.out')
        CACHIX_AUTH_TOKEN=$(echo "$stdin" | jq -r '.secrets.cachix_token')
        export CACHIX_AUTH_TOKEN
        cachix push ${name} "$path" >&2
      '';
    });
}
