utils: {lib}: let
  inherit
    (lib)
    mkAction
    ;
in {
  mkGithubStatus = {
    owner,
    repo,
  }:
    mkAction (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = [
          pkgs.curl
          pkgs.gnused
          pkgs.jq
        ];
        text = ''
          input=$(cat)

          build=$(echo "$input" | jq '.input.build' -r)
          data=$(echo "$input" | jq '.input.data' -r)
          evaluation=$(echo "$input" | jq '.input.evaluation' -r)
          flake_locked=$(echo "$input" | jq '.input.flake_locked' -r)
          job=$(echo "$input" | jq '.input.job' -r)
          jobset=$(echo "$input" | jq '.input.jobset' -r)
          project=$(echo "$input" | jq '.input.project' -r)
          status=$(echo "$input" | jq '.input.status' -r)

          token=$(echo "$input" | jq '.secrets.github_token' -r)

          rev=$(echo "$flake_locked" | sed 's/github:.*\/.*\/\(.*\)/\1/')
          target_url="$(echo "$data" | jq '.url' -r)/builds/$build"
          context="Typhon: $job"
          description="$project:$jobset:$evaluation:$job"
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

          curl -s \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $token" \
            "https://api.github.com/repos/${owner}/${repo}/statuses/$rev" \
            -d "{\"state\":\"$state\",\"target_url\":\"$target_url\",\"description\":\"$description\",\"context\":\"$context\"}" \
            -k >&2
        '';
      });
}
