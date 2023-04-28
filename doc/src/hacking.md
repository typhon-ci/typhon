# Hacking

Typhon is written in Rust. It consists of three packages: `typhon` is the
backend server, `typhon-webapp` is the frontend application, and `typhon-types`
is a common library shared between the two.

## Development environment

Simply run `nix develop` at the root of the project to enter the development
shell.

## Backend

The backend uses [Actix](https://actix.rs/) for the web server and
[Diesel](https://diesel.rs/) for the database management.

To run the server, go to the `typhon/` directory and run:

```shell
cargo run -- -p $(echo -n password | sha256sum | head -c 64) -j null -w ""
```

The server will be available at `http://localhost:8000`.

## Frontend

The frontend consists of a webapp written with [Seed](https://seed-rs.org/). It
can be built and tested with [Trunk](https://trunkrs.dev/). To run the webapp,
go to the `typhon-webapp/` directory and run `npm install` then `trunk serve`.
The webapp will be available at `http://localhost:8080`.
