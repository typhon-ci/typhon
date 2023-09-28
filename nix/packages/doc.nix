{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit sources system;},
}:
pkgs.stdenv.mkDerivation {
  name = "typhon-doc";
  src = ../../doc;
  nativeBuildInputs = [pkgs.mdbook];
  buildPhase = "mdbook build";
  installPhase = "cp -r book $out";
}
