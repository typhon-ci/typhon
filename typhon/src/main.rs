use actix_web::{App, HttpServer};
use clap::Parser;
use diesel::connection::SimpleConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = typhon::Args::parse();

    // Setup logger
    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(usize::from(args.verbose))
        .timestamp(args.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    // Enable foreign key support
    let _ = typhon::connection()
        .await
        .batch_execute("PRAGMA foreign_keys = ON");

    // Run diesel migrations
    let _ = typhon::connection()
        .await
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    // Run actix server
    let actix = HttpServer::new(|| App::new().configure(typhon::api::config))
        .bind(("127.0.0.1", 8000))?
        .run();

    // Graceful shutdown
    tokio::select! {
        _ = actix => panic!(),
        _ = typhon::shutdown() => eprintln!("Good bye!"),
    }

    Ok(())
}
