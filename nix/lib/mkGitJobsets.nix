utils: lib: {
  mkGitJobsets = {
    url,
    flake ? true,
  }:
    lib.mkActionScript {
      mkPath = system: let
        pkgs = utils.pkgs.${system};
      in [
        pkgs.git
        pkgs.gnused
        pkgs.jq
      ];
      mkScript = system: ''
        heads=$(git ls-remote --heads ${url} | sed 's/.*refs\/heads\/\(.*\)/\1/')
        echo null | jq --arg heads "$heads" '$heads
          | split("\n")
          | map({(.): {
              "url": ("git+${url}?ref=" + .),
              "flake": ${utils.lib.boolToString flake}
            }})
          | add'
      '';
    };
}
