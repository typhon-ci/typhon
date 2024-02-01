utils: lib: {
  mkGithubStatus = {
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

        token=$(echo "$input" | jq '.secrets.github_token' -r)

        rev=$(nix eval --json --expr "builtins.parseFlakeRef \"$url\"" | jq '.rev' -r)
        target_url="${typhon_url}/evaluation/$evaluation/$system/$job"
        context="Typhon: $system / $job"

        case $status in
          "error")
            state="failure"
            ;;
          "pending")
            state="pending"
            ;;
          "success")
            state="success"
            ;;
          *)
            state="error"
            ;;
        esac

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
          -X POST \
          -H "Accept: application/vnd.github+json" \
          -H "Authorization: Bearer $token" \
          "https://api.github.com/repos/${owner}/${repo}/statuses/$rev" \
          -d "$payload" \
          >&2
      '';
    };
}
