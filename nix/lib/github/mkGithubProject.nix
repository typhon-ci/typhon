_: lib: let
  inherit
    (lib)
    mkProject
    ;
  inherit
    (lib.github)
    githubWebhook
    mkGithubJobsets
    mkGithubStatus
    ;
in {
  mkGithubProject = {
    owner,
    repo,
    secrets,
    typhon_url,
    title ? repo,
    description ? "",
    homepage ? "https://github.com/${owner}/${repo}",
    legacy ? false,
  }:
    mkProject {
      meta = {inherit title description homepage;};
      actions = {
        jobsets = mkGithubJobsets {inherit owner repo legacy;};
        begin = mkGithubStatus {inherit owner repo typhon_url;};
        end = mkGithubStatus {inherit owner repo typhon_url;};
        webhook = githubWebhook;
      };
      inherit secrets;
    };
}
