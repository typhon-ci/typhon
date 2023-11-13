use actix_web::{App, HttpServer};
use clap::Parser;

mod api;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = typhon_lib::Args::parse();

    // Setup logger
    stderrlog::new()
        .module(module_path!())
        .quiet(args.quiet)
        .verbosity(usize::from(args.verbose))
        .timestamp(args.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    // Run actix server
    HttpServer::new(move || App::new().configure(api::config))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await?;

    // Graceful shutdown (FIXME)

    Ok(())
}
