use actix_files::Files;
use actix_web::*;
use clap::Parser;

mod api;

use actix_cors::Cors;
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

use typhon_webapp::app::App;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = typhon_lib::Args::parse();

    // Setup logger
    stderrlog::new()
        .module(module_path!())
        .module("typhon_lib")
        .quiet(args.quiet)
        .verbosity(usize::from(args.verbose))
        .timestamp(args.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    // Run actix server
    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);
    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        let cors = Cors::permissive(); // TODO: configure
        App::new()
            // webapp
            .route("/leptos/{tail:.*}", leptos_actix::handle_server_fns())
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", site_root))
            .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .app_data(web::Data::new(leptos_options.to_owned()))
            // server
            .configure(api::config)
            .wrap(cors)
    })
    .bind(&addr)?
    .run()
    .await?;

    // Graceful shutdown (FIXME)

    Ok(())
}
