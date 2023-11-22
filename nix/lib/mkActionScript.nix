utils: lib: {
  mkActionScript = {
    mkPath ? pkgs: [],
    script,
  }:
    lib.eachSystem (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = mkPath pkgs;
        text = script;
      });
}
