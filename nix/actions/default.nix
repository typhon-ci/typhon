{pkgs}: rec {
  mkProject = args @ {
    actions ? {},
    meta ? {},
    ...
  }: let
    linkIfExists = attrset: key:
      if attrset ? ${key}
      then "ln -s ${attrset.${key}} ${key}"
      else "";
  in {
    inherit meta;
    actions = pkgs.stdenv.mkDerivation {
      name = "actions";
      phases = ["installPhase"];
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

  mkAction = {
    packages,
    script,
  }: let
    path =
      pkgs.lib.foldr (a: b: "${a}/bin:${b}") "${pkgs.coreutils}/bin" packages;
  in
    pkgs.writeShellScript "action" ''
      export PATH=${path}
      set -euo pipefail
      ${script}
    '';

  gitJobsets = url:
    mkAction {
      packages = [pkgs.git pkgs.gnused pkgs.jq];
      script = ''
        heads=$(git ls-remote --heads ${url} | sed 's/.*refs\/heads\/\(.*\)/\1/')
        cmd=""
        for head in $heads
        do
          cmd="$cmd . += {\"$head\": { \"flake\": \"git+${url}?ref=$head\" } } |"
        done
        array=$(echo "{}" | jq "$cmd .")
        echo $array
      '';
    };

  mkGithubProject = {
    owner,
    repo,
    secrets,
    title ? repo,
    description ? "",
    homepage ? "https://github.com/${owner}/${repo}",
  }: let
    mkGhAction = script:
      mkAction {
        packages = [pkgs.curl pkgs.gnused pkgs.jq];
        inherit script;
      };
    githubStatus = mkGhAction ''
      input=$(cat)

      build=$(echo $input | jq '.input.build' -r)
      data=$(echo $input | jq '.input.data' -r)
      evaluation=$(echo $input | jq '.input.evaluation' -r)
      flake=$(echo $input | jq '.input.flake' -r)
      flake_locked=$(echo $input | jq '.input.flake_locked' -r)
      job=$(echo $input | jq '.input.job' -r)
      jobset=$(echo $input | jq '.input.jobset' -r)
      project=$(echo $input | jq '.input.project' -r)
      status=$(echo $input | jq '.input.status' -r)

      token=$(echo $input | jq '.secrets.github_token' -r)

      rev=$(echo $flake_locked | sed 's/github:.*\/.*\/\(.*\)/\1/')
      target_url="$(echo $data | jq '.url' -r)/builds/$build"
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
        https://api.github.com/repos/${owner}/${repo}/statuses/$rev \
        -d "{\"state\":\"$state\",\"target_url\":\"$target_url\",\"description\":\"$description\",\"context\":\"$context\"}" \
        -k >&2
    '';
    githubJobsets = mkGhAction ''
      input=$(cat)

      token=$(echo $input | jq '.secrets.github_token' -r)

      curl -s \
        -H "Accept: application/vnd.github+json" \
        -H "Authorization: Bearer $token" \
        https://api.github.com/repos/${owner}/${repo}/branches \
        -k \
        | jq --arg o "${owner}" --arg r "${repo}" 'map({ key: .name, value: { "flake": ("github:" + $o + "/" + $r + "/" + .name) }}) | from_entries'
    '';
  in
    mkProject {
      meta = {inherit title description homepage;};
      actions = {
        jobsets = githubJobsets;
        begin = githubStatus;
        end = githubStatus;
      };
      inherit secrets;
    };
}
