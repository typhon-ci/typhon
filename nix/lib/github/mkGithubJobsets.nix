utils: lib: {
  mkGithubJobsets = {
    owner,
    repo,
    flake ? true,
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

        token=$(echo "$input" | jq '.secrets.github_token' -r)

        curl -sf \
          --cacert ${utils.pkgs.${system}.cacert}/etc/ssl/certs/ca-bundle.crt \
          -H "Accept: application/vnd.github+json" \
          -H "Authorization: Bearer $token" \
          https://api.github.com/repos/${owner}/${repo}/branches \
          | jq '.
            | map({ (.name): {
                "url": ("github:${owner}/${repo}/" + .name),
                "flake": ${utils.lib.boolToString flake}
              }})
            | add'
      '';
    };
}
