use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

mod common;
mod group;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let cors = CorsLayer::permissive();
    let app = Router::new().nest("/group", group::router()).layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server is running on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
