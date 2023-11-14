# Hacking

Typhon is written in Rust. It consists of four packages:
- `typhon-lib` is the core logic of Typhon
- `typhon-webapp` is the frontend application
- `typhon-types` is a common library shared between the two
- `typhon` is the server and the main package

## Development environment

This documentation assumes that you are using Nix, so you can simply run
`nix-shell` at the root of the project to enter the development environment.
Experimental features "nix-command" and "flakes" need to be enabled in your Nix
configuration for the server to run properly. Nix >= 2.18 is also required but
it is provided by the Nix shell.

## Dependencies

Typhon uses [Actix](https://actix.rs/) for the web server and
[Diesel](https://diesel.rs/) for the database management. The webapp is written
with [Leptos](https://leptos.dev/). Typhon is built with `cargo-leptos`.

## Building

To build Typhon, go to the root of the project and run:

```shell
nix-shell
cargo leptos build
```

## Testing

To run Typhon, create `/nix/var/nix/gcroots/typhon/` and make sure that you
have write access to the directory. Then go to the root of the project and run:

```shell
nix-shell
export PASSWORD=$(echo -n "password" | sha256sum | head -c 64)
export VERBOSE=3
cargo leptos serve
```

The server will be available at `http://localhost:3000`.

You can also run `cargo leptos watch` to re-compile the server automatically at
each modification of the code.
