utils: lib: {
  mkGiteaStatus = {
    instance,
    owner,
    repo,
    typhon_url,
  }:
    lib.mkActionScript {
      mkPath = system: let
        pkgs = utils.pkgs.${system};
      in [
        pkgs.curl
        pkgs.jq
        pkgs.nix
      ];
      mkScript = system: ''
        input=$(cat)

        evaluation=$(echo "$input" | jq '.input.evaluation' -r)
        job=$(echo "$input" | jq '.input.job' -r)
        status=$(echo "$input" | jq '.input.status' -r)
        system=$(echo "$input" | jq '.input.system' -r)
        url=$(echo "$input" | jq '.input.url' -r)

        token=$(echo "$input" | jq '.secrets.gitea_token' -r)

        rev=$(nix eval --json --expr "builtins.parseFlakeRef \"$url\"" | jq '.rev' -r)
        system_encoded=$(echo -n "$system" | jq '@uri' -sRr)
        job_encoded=$(echo -n "$job" | jq '@uri' -sRr)
        target_url="${typhon_url}/evaluation/$evaluation/$system_encoded/$job_encoded"
        context="Typhon: $system / $job"
        state="$status"

        payload=$(echo null | jq \
          --arg state "$state" \
          --arg target_url "$target_url" \
          --arg context "$context" \
          '{
            "state": $state,
            "target_url": $target_url,
            "description": null,
            "context": $context
           }' \
        )

        curl -sf \
          --cacert ${utils.pkgs.${system}.cacert}/etc/ssl/certs/ca-bundle.crt \
          --json "$payload" \
          -H "Accept: application/json" \
          -H "Authorization: token $token" \
          "https://${instance}/api/v1/repos/${owner}/${repo}/statuses/$rev" \
          >&2
      '';
    };
}