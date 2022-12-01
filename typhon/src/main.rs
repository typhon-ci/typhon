use actix_web::{App, HttpServer};
use clap::Parser;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use std::sync::Mutex;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

/// Typhon, Nix-based continuous integration
#[derive(Parser, Debug)]
#[command(name = "Typhon")]
#[command(about = "Nix-based continuous integration", long_about = None)]
struct Args {
    /// Hashed password
    #[arg(long, short)]
    password: String,

    /// Silence all output
    #[arg(long, short)]
    quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc)
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Timestamp (sec, ms, ns, none)
    #[arg(long, short)]
    ts: Option<stderrlog::Timestamp>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let settings = typhon::Settings {
        hashed_password: args.password.clone(),
    };

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Initialize Typhon's state
    let _ = typhon::SETTINGS.set(settings);
    let _ = typhon::EVALUATIONS.set(typhon::tasks::Tasks::new());
    let _ = typhon::BUILDS.set(typhon::tasks::Tasks::new());
    let _ = typhon::JOBS.set(typhon::tasks::Tasks::new());
    let _ = typhon::CONNECTION.set(Mutex::new(conn));
    let _ = typhon::LISTENERS.set(Mutex::new(typhon::listeners::Listeners::new()));

    // Enable foreign key support
    let _ = typhon::connection().batch_execute("PRAGMA foreign_keys = ON");

    // Run diesel migrations
    let _ = typhon::connection()
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(usize::from(args.verbose))
        .timestamp(args.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    // Run actix server
    HttpServer::new(|| App::new().configure(typhon::api::config))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await
}
