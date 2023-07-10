{
  bubblewrap,
  coreutils,
  diesel-cli,
  nix,
  pkg-config,
  sqlite,
  stdenv,
  typhon,
  typhon-api-client-test,
}:
stdenv.mkDerivation {
  name = "Test API";
  phases = ["configurePhase" "installPhase"];
  DATABASE_URL = "/tmp/typhon.sqlite";
  configurePhase = ''
    export HOME=$(mktemp -d)
    mkdir -p ~/.config/nix
    echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
  '';
  installPhase = ''
    # start Typhon server
    typhon -p $(echo -n password | sha256sum | head -c 64) -j null -w "" &
    sleep 1

    # run the test client
    PROJECT_DECL="path:${../tests/empty}" typhon-api-client-test

    # kill the server and creates $out
    kill %1 && touch $out
  '';
  nativeBuildInputs = [
    typhon-api-client-test
    typhon
    coreutils
    bubblewrap
    diesel-cli
    pkg-config
    sqlite
    nix
  ];
}
