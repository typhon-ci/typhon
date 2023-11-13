utils: lib: {
  dummyWebhook = lib.eachSystem (system: let
    pkgs = utils.pkgs.${system};
  in
    pkgs.writeShellApplication {
      name = "action";
      runtimeInputs = [pkgs.jq];
      text = "cat | jq '.input.body' -r";
    });
}
