_: lib: let
  inherit
    (lib)
    dummyStatus
    dummyWebhook
    mkDummyAction
    mkProject
    mkSimpleJobsets
    ;
in {
  mkSimpleProject = {
    url,
    flake ? true,
    refs ? ["main"],
    title ? "",
    description ? "",
    homepage ? "",
  }:
    mkProject {
      meta = {inherit title description homepage;};
      actions = {
        jobsets = mkSimpleJobsets {inherit url flake refs;};
        begin = dummyStatus;
        end = dummyStatus;
        webhook = dummyWebhook;
      };
    };
}
