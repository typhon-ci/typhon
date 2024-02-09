_: lib: {
  mkProject = {
    instance,
    owner,
    repo,
    secrets,
    typhonUrl,
    title ? repo,
    description ? "",
    homepage ? "https://${instance}/${owner}/${repo}",
    flake ? true,
  } @ args:
    lib.common.mkProject (builtins.removeAttrs args ["instance"]
      // {
        inherit description flake homepage title;
        api = "${instance}/api/v1";
        authorizationKeyword = "token";
        tokenName = "gitea_token";
        urlPrefix = "git+https://${instance}/${owner}/${repo}?ref=";
        webhookSecretName = "gitea_webhook_secret";
      });
}
