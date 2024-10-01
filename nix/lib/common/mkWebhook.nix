utils: lib: {
  mkWebhook =
    {
      flake,
      urlPrefix,
      webhookSecretName,
    }:
    lib.builders.mkActionScript (
      { pkgs, system }:
      {
        path = [
          pkgs.jq
          pkgs.openssl
        ];
        script = ''
          stdin=$(cat)

          headers=$(echo "$stdin" | jq '.input.headers')
          body=$(echo "$stdin" | jq -r '.input.body')
          secret=$(echo "$stdin" | jq -r '.secrets.${webhookSecretName}')

          signatureSent=$(echo "$headers" | jq -r '."x-hub-signature-256"' | tail -c +8)
          signatureComputed=$(echo -n "$body" | openssl dgst -sha256 -hmac "$secret" | tail -c +18)
          [ "$signatureSent" == "$signatureComputed" ]

          event=$(echo "$headers" | jq -r '."x-github-event"')
          [ "$event" == "push" ] || { echo '[]'; exit 0; }

          ref=$(echo "$body" | jq -r '.ref|split("/")|.[2:]|join("/")')
          before=$(echo "$body" | jq -r '.before')
          after=$(echo "$body" | jq -r '.after')
          null="0000000000000000000000000000000000000000"

          if [ "$before" == "$null" ]
          then
            echo 'null' | jq --arg ref "$ref" '[{"command":"NewJobset", "name":$ref, "decl":{"flake":${utils.nixpkgsLib.boolToString flake},"url":("${urlPrefix}"+$ref)}}, {"command":"EvaluateJobset", "name":$ref}]'
          elif [ "$after" == "$null" ]
          then
            echo 'null' | jq --arg ref "$ref" '[{"command":"DeleteJobset", "name":$ref}]'
          else
            echo 'null' | jq --arg ref "$ref" '[{"command":"EvaluateJobset", "name":$ref}]'
          fi
        '';
      }
    );
}
