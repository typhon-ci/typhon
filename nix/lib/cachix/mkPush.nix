utils: lib: {
  mkPush =
    { name }:
    lib.builders.mkActionScript {
      mkPath =
        system:
        let
          pkgs = utils.pkgs.${system};
        in
        [
          pkgs.jq
          pkgs.cachix
        ];
      mkScript = system: ''
        stdin=$(cat)
        path=$(echo "$stdin" | jq -r '.input.out')
        CACHIX_AUTH_TOKEN=$(echo "$stdin" | jq -r '.secrets.cachix_token')
        export CACHIX_AUTH_TOKEN
        cachix push ${name} "$path" >&2
      '';
    };
}
