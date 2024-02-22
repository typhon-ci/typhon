utils: lib: {
  mkPages = {
    owner,
    repo,
    jobset ? "main",
    job,
    system ? "x86_64-linux",
    branch ? "gh-pages",
    customDomain ? null,
  }: let
    patches =
      if customDomain == null
      then []
      else [
        (
          system:
            utils.pkgs.${system}.runCommand "patch"
            {buildInputs = [utils.pkgs.${system}.git];}
            ''
              git init
              git config user.email "typhon@typhon-ci.org"
              git config user.name "Typhon"
              export GIT_AUTHOR_DATE="1970-01-01T00:00:00+0000"
              export GIT_COMMITTER_DATE="1970-01-01T00:00:00+0000"
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
        inherit jobset job system;
        action = lib.github.mkPushResult {
          inherit owner repo branch patches;
        };
      }
      {
        action = lib.builders.mkActionScript {
          mkScript = system: ''echo "Nothing to do" >&2'';
        };
      }
    ];
}
