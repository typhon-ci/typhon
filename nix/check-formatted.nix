{
  alejandra,
  rustToolchain,
  stdenv,
}:
stdenv.mkDerivation {
  name = "formatted";
  src = ../.;
  nativeBuildInputs = [
    alejandra
    rustToolchain
  ];
  buildPhase = ''
    alejandra -c .
    cargo fmt --check
  '';
  installPhase = "touch $out";
}
