utils: lib: {
  mkJobsets =
    {
      api,
      authorizationKeyword,
      flake,
      owner,
      repo,
      tokenName,
      urlPrefix,
    }:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [
          pkgs.curl
          pkgs.jq
        ];
        script = ''
          input=$(cat)

          token=$(echo "$input" | jq -r '.secrets.${tokenName}')

          curl -sf \
            --cacert ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt \
            -H "Accept: application/json" \
            -H "Authorization: ${authorizationKeyword} $token" \
            https://${api}/repos/${owner}/${repo}/branches \
            | jq '.
              | map({ (.name): {
                  "url": ("${urlPrefix}" + .name),
                  "flake": ${utils.nixpkgsLib.boolToString flake}
                }})
              | add'
        '';
      }
    );
}
