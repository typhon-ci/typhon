utils: lib: {
  schemas.typhonJobs = {
    version = 1;
    doc = ''
      Job declarations for [Typhon](https://typhon-ci.org/)
    '';
    allowIFD = false;
    inventory = output: {
      children = builtins.mapAttrs (name: derivation: {
        inherit derivation;
        evalChecks = { };
        forSystems = [ derivation.system ];
        isFlakeCheck = false;
        shortDescription = "";
        what = "Typhon job declaration";
      }) output;
    };
  };

  schemas.typhonProject = {
    version = 1;
    doc = ''
      Project declaration for [Typhon](https://typhon-ci.org/)";
    '';
    allowIFD = false;
    inventory = output: {
      evalChecks = { };
      shortDescription = "";
      what = "Typhon project declaration";
    };
  };
}
