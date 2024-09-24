utils: lib: {
  mkProject =
    args@{
      actions ? { },
      meta ? { },
    }:
    {
      inherit meta;
      actions = lib.eachSystem (
        system:
        let
          pkgs = utils.pkgs.${system};
          linkAction =
            name:
            if actions ? ${name} && actions.${name} ? ${system} then
              "ln -s ${actions.${name}.${system}}/bin/action ${name}"
            else
              "";
        in
        pkgs.runCommand "actions" { } ''
          mkdir $out
          cd $out
          ${linkAction "jobsets"}
          ${linkAction "begin"}
          ${linkAction "end"}
          ${linkAction "webhook"}
        ''
      );
    };
}
