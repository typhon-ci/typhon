use actix_web::{web, App, HttpServer};
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

    // Connect to the sqlite database
    let mut conn = typhon::POOL.get().unwrap();

    // Enable foreign key support
    let _ = conn.batch_execute("PRAGMA foreign_keys = ON");

    // Run diesel migrations
    let _ = conn
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    drop(conn);

    // Run actix server
    let actix = tokio::spawn(
        HttpServer::new(move || {
            App::new()
                .configure(typhon::api::config)
                .app_data(web::Data::new(typhon::POOL.clone()))
        })
        .disable_signals()
        .bind(("127.0.0.1", 8000))?
        .run(),
    );

    // Graceful shutdown
    let _ = tokio::signal::ctrl_c().await;
    actix.abort();
    typhon::shutdown().await;

    Ok(())
}
