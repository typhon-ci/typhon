utils: lib: {
  mkActionScript =
    f:
    lib.eachSystem (
      system:
      let
        pkgs = utils.pkgs.${system};
        values = f { inherit pkgs system; };
      in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = [ pkgs.coreutils ] ++ values.path or [ ];
        text = values.script;
        excludeShellChecks = [ "SC2015" ];
      }
    );
}
