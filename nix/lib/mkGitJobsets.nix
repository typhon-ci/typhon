utils: lib: let
  inherit
    (lib)
    eachSystem
    ;
in {
  mkGitJobsets = {
    url,
    legacy ? false,
  }:
    eachSystem (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = [
          pkgs.git
          pkgs.gnused
          pkgs.jq
        ];
        text = ''
          heads=$(git ls-remote --heads ${url} | sed 's/.*refs\/heads\/\(.*\)/\1/')
          echo null | jq --arg heads "$heads" '$heads
            | split("\n")
            | map({(.): {
                "url": ("git+${url}?ref=" + .),
                "legacy": ${utils.lib.boolToString legacy}
              }})
            | add'
        '';
      });
}
