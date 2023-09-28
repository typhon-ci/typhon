{
  sources ? import ../sources.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix {inherit sources system;},
  rust ? import ../rust.nix {inherit sources system;},
}:
pkgs.stdenv.mkDerivation {
  name = "formatted";
  src = ../..;
  nativeBuildInputs = [
    pkgs.alejandra
    rust.rustToolchain
  ];
  buildPhase = ''
    alejandra -c .
    cargo fmt --check
  '';
  installPhase = "touch $out";
}
