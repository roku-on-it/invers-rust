use std::fs::File;

use serde_json::Value;

mod constants;
mod http_handlers;
mod reverse_proxy;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let config_file = File::open("invers.json").unwrap();
    let config: Value = serde_json::from_reader(config_file).unwrap();

    let keys = config
        .as_object()
        .unwrap()
        .keys()
        .map(|x| x.as_str())
        .collect::<Vec<&str>>();

    for key in keys {
        match key {
            "http" => {
                tokio::spawn(reverse_proxy::setup_http_reverse_proxy(
                    config.get(key).unwrap().clone(),
                ));
            }
            "websocket" => {
                tokio::spawn(reverse_proxy::setup_websocket_reverse_proxy());
            }
            unknown_server_type => {
                eprintln!(
                    "Error: Unknown server type: {}",
                    unknown_server_type
                );

                std::process::exit(1);
            }
        }
    }

    let mut sig_term_signal = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    )?;

    // Block until we receive a SIGTERM signal.
    sig_term_signal.recv().await;

    println!("Received SIGTERM, shutting down invers");

    Ok(())
}
