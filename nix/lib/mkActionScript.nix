utils: lib: {
  mkActionScript = {
    mkPath ? pkgs: [],
    script ? "",
    mkScript ? system: script,
  }:
    lib.eachSystem (system: let
      pkgs = utils.pkgs.${system};
    in
      pkgs.writeShellApplication {
        name = "action";
        runtimeInputs = mkPath pkgs;
        text = mkScript system;
        excludeShellChecks = ["SC2015"];
      });
}
