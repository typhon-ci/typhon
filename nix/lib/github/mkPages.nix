_: lib: {
  mkPages = {
    owner,
    repo,
    jobset ? "main",
    job,
    system ? "x86_64-linux",
    branch ? "gh-pages",
  }:
    lib.compose.match [
      {
        inherit jobset job system;
        action = lib.github.mkPushResult {
          inherit owner repo branch;
        };
      }
      {
        action = lib.builders.mkActionScript {
          mkScript = system: ''echo "Nothing to do" >&2'';
        };
      }
    ];
}
