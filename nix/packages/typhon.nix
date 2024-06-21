{
  inputs ? import ../inputs.nix,
  system ? builtins.currentSystem or "unknown-system",
  pkgs ? import inputs.nixpkgs { inherit system; },
  craneLib ? inputs.crane.mkLib pkgs,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);

  args = {
    pname = "typhon";
    inherit (cargoToml.workspace.package) version;
    src = pkgs.lib.sourceByRegex ../.. [
      "Cargo.toml"
      "Cargo.lock"
      "typhon.*"
    ];
    RUSTC_BOOTSTRAP = 1;
  };

  leptos-toml = pkgs.writeText "leptos.toml" ''
    [[workspace.metadata.leptos]]
    name = "typhon"
    bin-package = "typhon"
    lib-package = "typhon-webapp"
    lib-features = ["hydrate"]
  '';

  typhon-main = pkgs.writeText "main.rs" ''
    fn main() {}
  '';

  cargoArtifacts = craneLib.buildDepsOnly (
    args
    // {
      buildPhaseCargoCommand = ''
        cat ${leptos-toml} >> Cargo.toml
        cat ${typhon-main} >> typhon/src/main.rs
        rm -r typhon/src/bin
        cargo check --locked
        cargo leptos build --release -vvv
      '';
      nativeBuildInputs = [
        pkgs.binaryen
        pkgs.cargo-leptos
        pkgs.lld
      ];
    }
  );

  nodeDependencies = (import ../npm-nix { inherit system pkgs; }).nodeDependencies;
in
craneLib.buildPackage (
  args
  // {
    inherit cargoArtifacts;
    nativeBuildInputs = [
      pkgs.cargo-leptos
      pkgs.sqlite.dev
      pkgs.binaryen
      pkgs.makeWrapper
    ];
    buildPhaseCargoCommand = "cargo leptos build --release -vvv";
    installPhaseCommand = ''
      mkdir -p $out/bin
      cp target/release/typhon $out/bin/
      cp -r target/site $out/bin/
      cp -r ${nodeDependencies}/lib/node_modules $out/bin/site
      chmod +w -R $out/bin/site/node_modules
      wrapProgram $out/bin/typhon --set LEPTOS_SITE_ROOT $out/bin/site
    '';
    TYPHON_FLAKE = ../../typhon-flake;
    doNotLinkInheritedArtifacts = true;
  }
)
