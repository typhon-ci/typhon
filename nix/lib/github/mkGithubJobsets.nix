utils: lib: {
  mkGithubJobsets = {
    owner,
    repo,
    flake ? true,
  }:
    lib.mkActionScript {
      mkPath = pkgs: [
        pkgs.curl
        pkgs.jq
      ];
      script = ''
        input=$(cat)

        token=$(echo "$input" | jq '.secrets.github_token' -r)

        curl -s \
          -H "Accept: application/vnd.github+json" \
          -H "Authorization: Bearer $token" \
          https://api.github.com/repos/${owner}/${repo}/branches \
          -k \ | jq '.
            | map({ (.name): {
                "url": ("github:${owner}/${repo}/" + .name),
                "flake": ${utils.lib.boolToString flake}
              }})
            | add'
      '';
    };
}
