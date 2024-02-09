utils: lib:
{
  giteaWebhook = lib.mkActionScript {
    mkPath = system: let
      pkgs = utils.pkgs.${system};
    in [
      pkgs.jq
      pkgs.openssl
    ];
    mkScript = system: ''
      input=$(cat)

      headers=$(echo "$input" | jq '.input.headers')
      body=$(echo "$input" | jq '.input.body' -r)
      secret=$(echo "$input" | jq '.secrets.gitea_webhook_secret' -r)

      signatureSent=$(echo "$headers" | jq '."x-gitea-signature"' -r)
      signatureComputed=$(echo -n "$body" | openssl dgst -sha256 -hmac "$secret" | tail -c +18)
      [ "$signatureSent" == "$signatureComputed" ]

      name=$(echo "$body" | jq '.ref|split("/")|.[2:]|join("/")')
      before=$(echo "$body" | jq '.before' -r)
      after=$(echo "$body" | jq '.after' -r)
      null="0000000000000000000000000000000000000000"

      if [ "$before" == "$null" ]
      then
        echo "$name" | jq '[{"command":"UpdateJobsets"}, {"command":"EvaluateJobset","name":.}]'
      elif [ "$after" == "$null" ]
      then
        echo 'null' | jq '[{"command":"UpdateJobsets"}]'
      else
        echo "$name" | jq '[{"command":"EvaluateJobset","name":.}]'
      fi
    '';
  };
}
