utils: lib: {
  mkActionScript = {
    mkPath ? system: [],
    mkScript,
  }:
    lib.eachSystem (system:
      utils.pkgs.${system}.writeShellApplication {
        name = "action";
        runtimeInputs = mkPath system;
        text = mkScript system;
        excludeShellChecks = ["SC2015"];
      });
}
