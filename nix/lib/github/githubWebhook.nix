utils: lib: let
  inherit
    (lib)
    eachSystem
    ;
in {
  githubWebhook = eachSystem (system: let
    pkgs = utils.pkgs.${system};
  in
    pkgs.writeShellApplication {
      name = "action";
      runtimeInputs = [
        pkgs.curl
        pkgs.jq
        pkgs.openssl
      ];
      text = ''
        input=$(cat)

        headers=$(echo "$input" | jq '.input.headers')
        body=$(echo "$input" | jq '.input.body' -r)
        secret=$(echo "$input" | jq '.secrets.github_webhook_secret' -r)

        event=$(echo "$headers" | jq '."x-github-event"' -r)
        [ "$event" == "push" ]

        signatureSent=$(echo "$headers" | jq '."x-hub-signature-256"' -r | tail -c +8)
        signatureComputed=$(echo -n "$body" | openssl dgst -sha256 -hmac "$secret" | tail -c +18)
        [ "$signatureSent" == "$signatureComputed" ]

        echo null | jq --argjson body "$body" '[]
          | if $body.created or $body.deleted then . + [{"command":"UpdateJobsets"}] else . end
          | if $body.deleted | not then . + [{"command":"EvaluateJobset","jobset":$body.ref|split("/")|.[2]}] else . end
          | .'
      '';
    });
}
