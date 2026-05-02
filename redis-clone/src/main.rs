mod error;
mod protocol;
mod store;
mod command;
mod server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = store::shared_store();
    let addr = "127.0.0.1:6379".parse().expect("invalid address");
    if let Err(e) = server::listener::run(addr, store).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
