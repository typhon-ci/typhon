{
  stdenv,
  mdbook,
}:
stdenv.mkDerivation {
  name = "typhon-doc";
  src = ../doc;
  nativeBuildInputs = [mdbook];
  buildPhase = "mdbook build";
  installPhase = "cp -r book $out";
}
