utils: lib: let
  inherit
    (lib)
    eachSystem
    ;
in {
  mkGithubStatus = {
    owner,
    repo,
  }:
    eachSystem (system: let
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
          flake_locked=$(echo "$input" | jq '.input.flake_locked' -r)
          job=$(echo "$input" | jq '.input.job' -r)
          status=$(echo "$input" | jq '.input.status' -r)
          system=$(echo "$input" | jq '.input.system' -r)

          token=$(echo "$input" | jq '.secrets.github_token' -r)

          rev=$(echo "$flake_locked" | sed 's/github:.*\/.*\/\(.*\)/\1/')
          target_url="$(echo "$data" | jq '.url' -r)/builds/$build"
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

          curl -s \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $token" \
            "https://api.github.com/repos/${owner}/${repo}/statuses/$rev" \
            -d "{\"state\":\"$state\",\"target_url\":\"$target_url\",\"description\":null,\"context\":\"$context\"}" \
            -k >&2
        '';
      });
}
