utils: lib: {
  mkPushResult =
    {
      owner,
      repo,
      branch,
      patches ? [ ],
    }:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [
          pkgs.jq
          pkgs.git
        ];
        script = ''
          stdin=$(cat)

          input=$(echo "$stdin" | jq -r '.input | ${utils.jqJsonToBashArray}')
          declare -A input="($input)"

          [ "''${input[status]}" == "success" ] || { echo "Unsuccessful build: nothing to push" >&2; exit 0; }

          token=$(echo "$stdin" | jq -r '.secrets.github_token')

          cp -r "''${input[out]}" out
          chmod -R +w out
          cd out

          git init
          ${lib.git.config}
          git checkout -b ${branch}

          git add .
          git commit -m "''${input[out]}"

          ${utils.nixpkgsLib.concatMapStrings (patch: ''
            git am < ${patch system}
          '') patches}

          export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          git remote add origin "https://x-access-token:$token@github.com/${owner}/${repo}"
          git push -f -u origin ${branch}
        '';
      }
    );
}
