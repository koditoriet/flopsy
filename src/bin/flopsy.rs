use clap::Parser;
use flopsy::{args::Args, proxy::Proxy};

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let proxy = Proxy::create(args);
    proxy.run().await;
}
