{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit sources system;},
}: let
  webapp = import ./webapp.nix {inherit sources system;};

  tarball = pkgs.stdenv.mkDerivation {
    name = "source.tar.gz";
    src = ../..;
    buildPhase = ''
      tar -czf $out \
        --sort=name \
        --transform 's/^/typhon\//' \
        .
    '';
  };
in
  pkgs.callPackage (
    {
      webroot ? "",
      api_url ? "http://127.0.0.1:8000/api",
    }: let
      settings = builtins.toJSON {inherit api_url;};
    in
      pkgs.stdenv.mkDerivation {
        name = "typhon-webroot";
        src = webapp;
        buildPhase = ''
          substituteInPlace ./index.html --replace \
              'WEBROOT' \
              '${webroot}/'
          substituteInPlace ./index.html --replace \
              '<script type="application/json" id="settings">null</script>' \
              '<script type="application/json" id="settings">${settings}</script>'
          cp ${tarball} source.tar.gz
        '';
        installPhase = ''
          mkdir -p $out${webroot}
          mv * $out${webroot}
        '';
      }
  ) {}
