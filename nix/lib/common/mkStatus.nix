utils: lib: {
  mkStatus = {
    api,
    authorizationKeyword,
    owner,
    repo,
    tokenName,
    typhonUrl,
  }:
    lib.builders.mkActionScript {
      mkPath = system: let
        pkgs = utils.pkgs.${system};
      in [
        pkgs.curl
        pkgs.jq
        pkgs.nix
      ];
      mkScript = system: ''
        stdin=$(cat)

        input=$(echo "$stdin" | jq -r '.input | ${utils.jqJsonToBashArray}')
        declare -A input="($input)"

        token=$(echo "$stdin" | jq -r '.secrets.${tokenName}')

        rev=$(HOME=$(pwd) nix --extra-experimental-features "nix-command flakes" eval --json --expr "builtins.parseFlakeRef \"''${input[url]}\"" | jq -r '.rev')
        system_encoded=$(echo -n "''${input[system]}" | jq '@uri' -sRr)
        job_encoded=$(echo -n "''${input[job]}" | jq '@uri' -sRr)
        target_url="${typhonUrl}/evaluation/''${input[evaluation]}/$system_encoded/$job_encoded"
        context="Typhon: ''${input[system]} / ''${input[job]}"
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
          --cacert ${utils.pkgs.${system}.cacert}/etc/ssl/certs/ca-bundle.crt \
          --json "$payload" \
          -H "Accept: application/json" \
          -H "Authorization: ${authorizationKeyword} $token" \
          "https://${api}/repos/${owner}/${repo}/statuses/$rev" \
          >&2
      '';
    };
}
