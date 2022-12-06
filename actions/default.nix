{ pkgs }: rec {

  mkProject = args@{ actions ? { }, meta ? { }, ... }:
    let
      linkIfExists = attrset: key:
        if attrset ? ${key} then "ln -s ${attrset.${key}} ${key}" else "";
    in {
      inherit meta;
      actions = pkgs.stdenv.mkDerivation {
        name = "actions";
        phases = [ "installPhase" ];
        installPhase = ''
          mkdir $out
          cd $out
          ${linkIfExists actions "jobsets"}
          ${linkIfExists actions "begin"}
          ${linkIfExists actions "end"}
          ${linkIfExists args "secrets"}
        '';
      };
    };

  mkAction = { packages, script }:
    let
      path =
        pkgs.lib.foldr (a: b: "${a}/bin:${b}") "${pkgs.coreutils}/bin" packages;
    in pkgs.writeShellScript "action" ''
      export PATH=${path}
      ${script}
    '';

  gitJobsets = url:
    mkAction {
      packages = [ pkgs.git pkgs.gnused pkgs.jq ];
      script = ''
        heads=$(git ls-remote --heads ${url} | sed 's/.*refs\/heads\/\(.*\)/\1/')
        for head in $heads
        do
          cmd="$cmd . += {\"$head\": { \"flake\": \"git+${url}?ref=$head\" } } |"
        done
        array=$(echo "{}" | jq "$cmd .")
        echo $array
      '';
    };

  mkGithubProject = { owner, repo, token, title ? repo, description ? ""
    , homepage ? "https://github.com/${owner}/${repo}" }:
    let
      parseInput = ''
        flake=$(echo $input | jq '.input.locked_flake' -r)
        project=$(echo $input | jq '.input.project' -r)
        jobset=$(echo $input | jq '.input.jobset' -r)
        evaluation=$(echo $input | jq '.input.evaluation' -r)
        job=$(echo $input | jq '.input.job' -r)

        ref=$(echo $flake | sed 's/github:.*\/.*\/\(.*\)/\1/')
        context="Typhon job: $job"
        description="$project:$jobset:$evaluation:$job"
      '';
      setGithubStatus = state: ''
        curl -X POST -H "Accept: application/vnd.github+json" -H "Authorization: Bearer $token" https://api.github.com/repos/${owner}/${repo}/statuses/$ref -d "{\"state\":\"${state}\",\"target_url\":\"https://typhon-ci.org\",\"description\":\"$description\",\"context\":\"$context\"}" -k
      '';
      mkScript = script:
        mkAction {
          packages = [ pkgs.jq pkgs.gnused pkgs.curl ];
          inherit script;
        };
    in mkProject {
      meta = { inherit title description homepage; };
      actions = {
        jobsets = mkScript ''
          input=$(cat)

          token=$(echo $input | jq '.secrets' -r)

          curl \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $token" \
            https://api.github.com/repos/${owner}/${repo}/branches \
            -k \
            | jq --arg o "${owner}" --arg r "${repo}" 'map({ key: .name, value: { "flake": ("github:" + $o + "/" + $r + "/" + .name) }}) | from_entries'
        '';
        begin = mkScript ''
          input=$(cat)

          ${parseInput}
          token=$(echo $input | jq '.secrets' -r)

          ${setGithubStatus "pending"}
        '';
        end = mkScript ''
          input=$(cat)

          ${parseInput}
          status=$(echo $input | jq '.input.status' -r)
          token=$(echo $input | jq '.secrets' -r)

          case $status in
            "error")
              ${setGithubStatus "failure"}
              ;;
            "success")
              ${setGithubStatus "success"}
              ;;
            *)
              ${setGithubStatus "error"}
              ;;
          esac
        '';
      };
      secrets = token;
    };

}
