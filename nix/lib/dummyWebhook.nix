utils: lib: let
  inherit
    (lib)
    eachSystem
    ;
in {
  dummyWebhook = eachSystem (system: let
    pkgs = utils.pkgs.${system};
  in
    pkgs.writeShellApplication {
      name = "action";
      runtimeInputs = [pkgs.jq];
      text = "cat | jq '.input.body' -r";
    });
}
