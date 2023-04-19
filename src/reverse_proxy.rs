use invers_http::HttpServer;
use serde_json::Value;

use crate::http_handlers::handle_proxy;

pub async fn setup_http_reverse_proxy(http_config: Value) {
    let hostnames = http_config
        .as_object()
        .unwrap()
        .keys()
        .map(|x| x.as_str())
        .collect::<Vec<&str>>();

    if hostnames.is_empty() {
        eprintln!("Could not find any hostname for HTTP config. Must define at least one hostname in config file. (e.g. \"example.com\", \"localhost\", etc.)");

        std::process::exit(1);
    }

    for hostname in hostnames {
        let ports = http_config.get(hostname).unwrap().get("listen_on_ports");

        match ports {
            Some(ports) => {
                for port in ports.as_array().unwrap() {
                    let port =
                        port.to_string().parse::<u16>().unwrap_or_else(|_| {
                            eprintln!("Port must be a number between 0 and 65535, received: {}", port);

                            std::process::exit(1);
                        });

                    let duplicate_port_exists = ports
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|&n| *n == port)
                        .count()
                        > 1;

                    if duplicate_port_exists {
                        eprintln!("Duplicate port found for {}:{}. Must define only one port per hostname", hostname, port);

                        std::process::exit(1);
                    }

                    let mut parsed_hostname = hostname;

                    // If the hostname is localhost, changing it to
                    // 127.0.0.1 because providing duplicate ports for the same
                    // hostname also binds to [::1]:port which ends up calling the handler twice.
                    if hostname == "localhost" {
                        parsed_hostname = "127.0.0.1";
                    }

                    let addr = format!("{}:{}", parsed_hostname, port);

                    println!("Setting up HTTP reverse proxy on {}", addr);

                    let mut http_server = HttpServer::new(addr.as_str()).await;

                    http_server.add_handler(Box::new(move |stream| {
                        Box::pin(handle_proxy(stream))
                    }));

                    tokio::spawn(async move {
                        http_server.run().await;
                    });
                }
            }
            None => {
                eprintln!("Could not find any ports for hostname {}. Must define at least one port in config file. (e.g. 80, 443, etc.)", hostname);

                std::process::exit(1);
            }
        }
    }
}

pub async fn setup_websocket_reverse_proxy() {}
