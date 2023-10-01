utils: lib: let
  inherit
    (lib)
    eachSystem
    ;
in {
  mkGithubStatus = {
    owner,
    repo,
    typhon_url,
  }:
    eachSystem (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = [
          pkgs.curl
          pkgs.jq
          pkgs.nix
        ];
        text = ''
          input=$(cat)

          url_locked=$(echo "$input" | jq '.input.url_locked' -r)
          job=$(echo "$input" | jq '.input.job' -r)
          status=$(echo "$input" | jq '.input.status' -r)
          system=$(echo "$input" | jq '.input.system' -r)

          token=$(echo "$input" | jq '.secrets.github_token' -r)

          rev=$(nix eval --json --expr "builtins.parseFlakeRef \"$url_locked\"" | jq '.rev' -r)
          target_url="${typhon_url}" # TODO: more precise target url
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

          curl -s \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $token" \
            "https://api.github.com/repos/${owner}/${repo}/statuses/$rev" \
            -d "$payload" \
            -k >&2
        '';
      });
}
