use clap::Parser;

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

    // Initialize Typhon's state
    typhon::SETTINGS.set(settings).unwrap();
    typhon::EVALUATIONS
        .set(typhon::tasks::Tasks::new())
        .unwrap();
    typhon::BUILDS.set(typhon::tasks::Tasks::new()).unwrap();
    typhon::JOBS.set(typhon::tasks::Tasks::new()).unwrap();

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
