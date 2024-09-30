{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import inputs.nixpkgs { inherit system; },
}:
pkgs.stdenv.mkDerivation {
  name = "formatted";
  src = ../..;
  nativeBuildInputs = [
    pkgs.nixfmt-rfc-style
    pkgs.leptosfmt
    pkgs.cargo
    pkgs.rustfmt
  ];
  buildPhase = ''
    nixfmt -c .
    leptosfmt --check .
    cargo fmt --check
  '';
  installPhase = "touch $out";
}
