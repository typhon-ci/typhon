utils: lib: {
  mkStatus =
    {
      api,
      authorizationKeyword,
      owner,
      repo,
      tokenName,
      typhonUrl,
    }:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [
          pkgs.curl
          pkgs.jq
          pkgs.nix
        ];
        script = ''
          stdin=$(cat)

          input=$(echo "$stdin" | jq -r '.input | ${utils.jqJsonToBashArray}')
          declare -A input="($input)"

          token=$(echo "$stdin" | jq -r '.secrets.${tokenName}')

          rev=$(HOME=$(pwd) nix --extra-experimental-features "nix-command flakes" eval --json --expr "builtins.parseFlakeRef \"''${input[url]}\"" | jq -r '.rev')
          job_encoded=$(echo -n "''${input[job]}" | jq '@uri' -sRr)
          target_url="${typhonUrl}/evaluation/''${input[evaluation]}/$job_encoded"
          context="Typhon: ''${input[job]}"
          state="''${input[status]}"

          payload=$(echo 'null' | jq \
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
            --cacert ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt \
            --json "$payload" \
            -H "Accept: application/json" \
            -H "Authorization: ${authorizationKeyword} $token" \
            "https://${api}/repos/${owner}/${repo}/statuses/$rev" \
            >&2
        '';
      }
    );
}
