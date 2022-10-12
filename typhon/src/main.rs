use clap::Parser;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::env;
use std::sync::Mutex;

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
    webroot: Option<String>,

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

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let args = Args::parse();

    let settings = typhon::Settings {
        hashed_password: args.password.clone(),
        webroot: args.webroot.unwrap_or(String::new()),
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

    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(usize::from(args.verbose))
        .timestamp(args.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    let webroot = &typhon::SETTINGS.get().unwrap().webroot;

    let rocket = rocket::build()
        .mount(format!("{}/api", webroot), typhon::api::routes())
        .attach(typhon::api::CORS)
        .ignite()
        .await?;
    let _ = rocket.launch().await?;
    Ok(())
}
