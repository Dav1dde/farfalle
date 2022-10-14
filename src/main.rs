use axum::{
    routing::{get, post},
    Router,
};
use bpaf::Bpaf;

use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
struct Args {
    #[bpaf(env("FARFALLE_PATH"))]
    path: PathBuf,

    #[bpaf(env("FARFALLE_ADDR"), fallback(SocketAddr::from(([127, 0, 0, 1], 3000))))]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args().run();

    tracing_subscriber::fmt::init();

    let storage = farfalle::FilesystemStorage::new(args.path, farfalle::RandomIdGen::new(3));
    let theme: farfalle::Theme = serde_json::from_str(include_str!("../themes/default.json"))?;

    let app = Router::new()
        .route("/", get(farfalle::handler::root))
        .route("/", post(farfalle::handler::upload))
        .route("/:id", get(farfalle::handler::view))
        .layer(storage.into_extension())
        .layer(theme.into_extension());

    tracing::info!("listening on {}", args.addr);
    axum::Server::bind(&args.addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(elegant_departure::tokio::depart().on_termination())
        .await?;

    Ok(())
}
