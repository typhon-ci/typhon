utils: lib: {
  mkGitJobsets = {
    url,
    flake ? true,
  }:
    lib.mkActionScript {
      mkPath = pkgs: [
        pkgs.git
        pkgs.gnused
        pkgs.jq
      ];
      script = ''
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
