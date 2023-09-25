utils: lib: let
  inherit
    (lib)
    systems
    ;
in {
  mkProject = args @ {
    actions ? {},
    meta ? {},
    secrets ? null,
  }: {
    inherit meta;
    actions = utils.lib.genAttrs systems (
      system: let
        pkgs = utils.pkgs.${system};
        linkAction = name:
          if actions ? ${name} && actions.${name} ? ${system}
          then "ln -s ${actions.${name}.${system}}/bin/action ${name}"
          else "";
        linkSecrets =
          if secrets != null
          then "ln -s ${secrets} secrets"
          else "";
      in
        pkgs.runCommand "actions" {} ''
          mkdir $out
          cd $out
          ${linkAction "jobsets"}
          ${linkAction "begin"}
          ${linkAction "end"}
          ${linkAction "webhook"}
          ${linkSecrets}
        ''
    );
  };
}
