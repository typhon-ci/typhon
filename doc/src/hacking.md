# Hacking

Typhon is written in Rust. It consists of three packages: `typhon` is the
backend server, `typhon-webapp` is the frontend application, and `typhon-types`
is a common library shared between the two.

## Development environment

This documentation assumes that you are using Nix. Development environments are
provided by Nix shells in the different subdirectories of the project.
Experimental features "nix-command" and "flakes" need to be enabled in your Nix
configuration for the server to run properly. Nix >= 2.18 is also required but
it is provided by the Nix shell of the server.

## Backend

The backend uses [Actix](https://actix.rs/) for the web server and
[Diesel](https://diesel.rs/) for the database management.

To run the server, create `/nix/var/nix/gcroots/typhon/` and make sure that you
have write access to the directory. Then go to `typhon/` and run:

```shell
nix-shell
cargo run -- -p $(echo -n password | sha256sum | head -c 64) -w ""
```

The server will be available at `http://localhost:8000`.

## Frontend

The frontend consists of a webapp written with [Seed](https://seed-rs.org/). It
can be built and tested with [Trunk](https://trunkrs.dev/). To run the webapp,
go to `typhon-webapp/` and run:

```shell
nix-shell
npm install
trunk serve
```

The webapp will be available at `http://localhost:8080`.
