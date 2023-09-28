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
    title ? repo,
    description ? "",
    homepage ? "https://github.com/${owner}/${repo}",
  }:
    mkProject {
      meta = {inherit title description homepage;};
      actions = {
        jobsets = mkGithubJobsets {inherit owner repo;};
        begin = mkGithubStatus {inherit owner repo;};
        end = mkGithubStatus {inherit owner repo;};
        webhook = githubWebhook;
      };
      inherit secrets;
    };
}
