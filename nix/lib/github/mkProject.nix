_: lib: {
  mkProject =
    {
      deploy ? [ ],
      owner,
      repo,
      secrets,
      typhonUrl,
      title ? repo,
      description ? "",
      homepage ? "https://github.com/${owner}/${repo}",
      flake ? true,
    }@args:
    lib.common.mkProject (
      args
      // {
        inherit
          deploy
          description
          flake
          homepage
          title
          ;
        api = "api.github.com";
        authorizationKeyword = "Bearer";
        tokenName = "github_token";
        urlPrefix = "github:${owner}/${repo}/";
        webhookSecretName = "github_webhook_secret";
      }
    );
}
