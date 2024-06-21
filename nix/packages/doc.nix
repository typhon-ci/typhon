{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import inputs.nixpkgs { inherit system; },
}:
pkgs.stdenv.mkDerivation {
  name = "typhon-doc";
  src = ../../doc;
  nativeBuildInputs = [ pkgs.mdbook ];
  buildPhase = "mdbook build";
  installPhase = "cp -r book $out";
  passthru.typhonDist = true;
}
