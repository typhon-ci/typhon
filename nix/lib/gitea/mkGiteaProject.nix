utils: lib: let
  inherit
    (lib.gitea)
    giteaWebhook
    mkGiteaJobsets
    mkGiteaStatus
    ;
in {
  mkGiteaProject = {
    instance,
    owner,
    repo,
    secrets,
    typhon_url,
    title ? repo,
    description ? "",
    homepage ? "https://${instance}/${owner}/${repo}",
    flake ? true,
  }:
    lib.mkProject {
      meta = {inherit title description homepage;};
      actions = {
        jobsets = mkGiteaJobsets {inherit instance owner repo flake;};
        begin = mkGiteaStatus {inherit instance owner repo typhon_url;};
        end = mkGiteaStatus {inherit instance owner repo typhon_url;};
        webhook = giteaWebhook;
      };
      inherit secrets;
    };
}
