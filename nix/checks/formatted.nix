{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import ../nixpkgs.nix { inherit inputs system; },
  rust ? import ../rust.nix { inherit inputs system; },
}:
pkgs.stdenv.mkDerivation {
  name = "formatted";
  src = ../..;
  nativeBuildInputs = [
    pkgs.nixfmt-rfc-style
    rust.rustToolchain
    pkgs.leptosfmt
  ];
  buildPhase = ''
    nixfmt -c .
    leptosfmt --check .
    cargo fmt --check
  '';
  installPhase = "touch $out";
}
