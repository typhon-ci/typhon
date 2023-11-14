{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit inputs system;},
}:
pkgs.stdenv.mkDerivation {
  name = "typhon-doc";
  src = ../../doc;
  nativeBuildInputs = [pkgs.mdbook];
  buildPhase = "mdbook build";
  installPhase = "cp -r book $out";
  passthru.typhonDist = true;
}
