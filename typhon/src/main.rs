mod api;

use actix_files::Files;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::*;
use clap::Parser;
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

use typhon_webapp::App;

const RANDOM_KEY: &str = "random";

/// Typhon, Nix-based continuous integration
#[derive(Parser)]
#[command(name = "Typhon")]
pub struct Args {
    /// Path to a file containing the admin password
    #[arg(long, short, env)]
    pub password: String,

    /// Cookie secret
    #[arg(long, value_parser={|s: &str| -> Result<Key, String> {
        if RANDOM_KEY == s {
            return Ok(Key::generate());
        }
        let hex = hex::decode(s).map_err(|e| e.to_string())?;
        Key::try_from(&hex[..]).map_err(|e| e.to_string())
    }}, default_value=RANDOM_KEY, env)]
    pub cookie_secret: Key,

    /// Silence all output
    #[arg(long, short, env)]
    pub quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc)
    #[arg(long, short, action = clap::ArgAction::Count, env)]
    pub verbose: u8,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    let args = Args::parse();

    // Initialization
    let password = std::fs::read_to_string(args.password)?;
    std::env::set_var("PASSWORD", password);
    typhon_core::init();

    // Run actix server
    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);
    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                args.cookie_secret.clone(),
            ))
            .configure(api::config)
            .route("/leptos/{tail:.*}", leptos_actix::handle_server_fns())
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", site_root))
            .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .app_data(web::Data::new(leptos_options.to_owned()))
    })
    .bind(&addr)?
    .run()
    .await?;

    // Graceful shutdown
    typhon_core::shutdown().await;

    Ok(())
}
