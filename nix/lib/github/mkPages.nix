utils: lib: {
  mkPages =
    {
      owner,
      repo,
      jobset ? "main",
      job,
      branch ? "gh-pages",
      customDomain ? null,
    }:
    let
      patches =
        if customDomain == null then
          [ ]
        else
          [
            (
              system:
              utils.pkgs.${system}.runCommand "patch" { buildInputs = [ utils.pkgs.${system}.git ]; } ''
                git init
                ${lib.git.config}
                echo "${customDomain}" > CNAME
                git add CNAME
                git commit -m "add CNAME"
                git format-patch --root --stdout > $out
              ''
            )
          ];
    in
    lib.compose.match [
      {
        inherit jobset job;
        action = lib.github.mkPushResult {
          inherit
            owner
            repo
            branch
            patches
            ;
        };
      }
      {
        action = lib.builders.mkActionScript (
          { pkgs, system }:
          {
            script = ''echo "Nothing to do" >&2'';
          }
        );
      }
    ];
}
