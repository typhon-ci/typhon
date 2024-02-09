_: lib: {
  mkProject = {
    api,
    authorizationKeyword,
    description,
    flake,
    homepage,
    owner,
    repo,
    secrets,
    title,
    tokenName,
    typhonUrl,
    urlPrefix,
    webhookSecretName,
  }: let
    inherit (lib) common;
    jobsets = common.mkJobsets {
      inherit
        api
        authorizationKeyword
        flake
        owner
        repo
        tokenName
        urlPrefix
        ;
    };
    status = common.mkStatus {
      inherit
        api
        authorizationKeyword
        owner
        repo
        tokenName
        typhonUrl
        ;
    };
    webhook = common.mkWebhook {inherit webhookSecretName;};
  in
    lib.builders.mkProject {
      meta = {inherit title description homepage;};
      actions = {
        inherit jobsets webhook;
        begin = lib.compose.steps [
          {
            name = "Set status";
            action = status;
          }
        ];
        end = lib.compose.steps [
          {
            name = "Set status";
            action = status;
          }
        ];
      };
      inherit secrets;
    };
}
