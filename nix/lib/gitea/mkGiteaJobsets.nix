utils: lib:
{
  mkGiteaJobsets = {
    instance,
    owner,
    repo,
    flake,
  }:
    lib.mkActionScript {
      mkPath = system: let
        pkgs = utils.pkgs.${system};
      in [
        pkgs.curl
        pkgs.jq
      ];
      mkScript = system: ''
        input=$(cat)

        token=$(echo "$input" | jq '.secrets.gitea_token' -r)

        curl -sf \
          --cacert ${utils.pkgs.${system}.cacert}/etc/ssl/certs/ca-bundle.crt \
          -H "Accept: application/json" \
          -H "Authorization: token $token" \
          https://${instance}/api/v1/repos/${owner}/${repo}/branches \
          | jq '.
            | map({ (.name): {
                "url": ("git+https://${instance}/${owner}/${repo}?ref=" + .name),
                "flake": ${utils.lib.boolToString flake}
              }})
            | add'
      '';
    };
}
