utils: lib: {
  mkPushResult = {
    owner,
    repo,
    branch,
  }:
    lib.builders.mkActionScript {
      mkPath = system: let
        pkgs = utils.pkgs.${system};
      in [pkgs.jq pkgs.git];
      mkScript = system: ''
        stdin=$(cat)

        input=$(echo "$stdin" | jq -r '.input | ${utils.jqJsonToBashArray}')
        declare -A input="($input)"

        [ "''${input[status]}" == "success" ] || { echo "Unsuccessful build: nothing to push" >&2; exit 0; }

        token=$(echo "$stdin" | jq -r '.secrets.github_token')

        cp -r "''${input[out]}" out
        chmod -R +w out
        cd out

        git init
        git config user.email "typhon@typhon-ci.org"
        git config user.name "Typhon"

        git checkout -b ${branch}
        git add .
        export GIT_AUTHOR_DATE="1970-01-01T00:00:00+0000"
        export GIT_COMMITTER_DATE="1970-01-01T00:00:00+0000"
        git commit -m "''${input[out]}"

        git remote add origin "https://$token@github.com/${owner}/${repo}"
        git push -f -u origin ${branch}
      '';
    };
}
