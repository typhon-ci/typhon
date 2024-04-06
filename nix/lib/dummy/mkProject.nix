_: lib: {
  mkProject =
    {
      url,
      flake ? true,
      refs ? [ "main" ],
      title ? "",
      description ? "",
      homepage ? "",
    }:
    lib.builders.mkProject {
      meta = {
        inherit title description homepage;
      };
      actions = {
        jobsets = lib.dummy.mkJobsets { inherit url flake refs; };
        begin = lib.dummy.status;
        end = lib.dummy.status;
        webhook = lib.dummy.webhook;
      };
    };
}
