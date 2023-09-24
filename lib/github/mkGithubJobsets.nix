utils: {lib}: let
  inherit
    (lib)
    eachSystem
    ;
in {
  mkGithubJobsets = {
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
          pkgs.jq
        ];
        text = ''
          input=$(cat)

          token=$(echo "$input" | jq '.secrets.github_token' -r)

          curl -s \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $token" \
            https://api.github.com/repos/${owner}/${repo}/branches \
            -k \
            | jq --arg o "${owner}" --arg r "${repo}" 'map({ key: .name, value: { "flake": ("github:" + $o + "/" + $r + "/" + .name) }}) | from_entries'
        '';
      });
}
