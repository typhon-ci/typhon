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
      baseurl ? "127.0.0.1:8000/api",
      https ? false,
    }: let
      settings = pkgs.writeTextFile {
        name = "settings.json";
        text = builtins.toJSON {inherit baseurl https;};
      };
    in
      pkgs.stdenv.mkDerivation {
        name = "typhon-webroot";
        src = webapp;
        buildPhase = ''
          substituteInPlace ./index.html --replace "WEBROOT" "${webroot}/"
          cp ${settings} settings.json
          cp ${tarball} source.tar.gz
        '';
        installPhase = ''
          mkdir -p $out${webroot}
          mv * $out${webroot}
        '';
      }
  ) {}
