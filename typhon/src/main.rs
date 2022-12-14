use actix_web::{App, HttpServer};
use clap::Parser;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use tokio::sync::Mutex;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

/// Typhon, Nix-based continuous integration
#[derive(Parser, Debug)]
#[command(name = "Typhon")]
#[command(about = "Nix-based continuous integration", long_about = None)]
struct Args {
    /// Hashed password
    #[arg(long, short)]
    password: String,

    /// Webroot
    #[arg(long, short)]
    webroot: String,

    /// Json data for the jobs
    #[arg(long, short)]
    json: String,

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

    // Setup logger
    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(usize::from(args.verbose))
        .timestamp(args.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    // Initialize Typhon's state
    let settings = typhon::Settings {
        hashed_password: args.password.clone(),
        json: serde_json::from_str(&args.json).expect("failed to parse json"),
        webroot: args.webroot.clone(),
    };
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    typhon::SETTINGS
        .set(settings)
        .expect("failed to initialize state value");
    typhon::EVALUATIONS
        .set(typhon::tasks::Tasks::new())
        .expect("failed to initialize state value");
    typhon::BUILDS
        .set(typhon::tasks::Tasks::new())
        .expect("failed to initialize state value");
    typhon::JOBS
        .set(typhon::tasks::Tasks::new())
        .expect("failed to initialize state value");
    typhon::CONNECTION
        .set(typhon::Connection::new(conn))
        .expect("failed to initialize state value");
    typhon::LISTENERS
        .set(Mutex::new(typhon::listeners::Listeners::new()))
        .expect("failed to initialize state value");

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
    HttpServer::new(|| App::new().configure(typhon::api::config))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await
}
