fn print_help() {
    println!(
        "lsharp-lsp {}\nUsage: lsharp-lsp [--stdio] [--help] [--version]",
        env!("CARGO_PKG_VERSION")
    );
}

#[tokio::main]
async fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--help") | Some("-h") => {
            print_help();
        }
        Some("--version") | Some("-V") | Some("-v") => {
            println!("lsharp-lsp {}", env!("CARGO_PKG_VERSION"));
        }
        Some("--stdio") | None => {
            lsharp_lsp::run_server().await;
        }
        Some(arg) => {
            eprintln!("unknown option: {arg}");
            std::process::exit(2);
        }
    }
}
