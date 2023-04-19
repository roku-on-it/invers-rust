use std::fs::File;
use std::ops::DerefMut;
use std::sync::Arc;

use invers_http::utils::http_utils::extract_request_from_tcp_stream;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub async fn handle_proxy(stream: Arc<Mutex<TcpStream>>) {
    // TODO: This is a bit of a hack and slow to do every time we get a new request. We should
    // read the config file once and then pass it to the handler.
    let config_file = File::open("invers.json").unwrap();
    let config: Value = serde_json::from_reader(config_file).unwrap();
    let http_config = config.get("http").unwrap();

    // TODO: This is also a bit of a hack, but it works for now. We need to find a better way to get the
    // hostname(s) from the config.
    let hostname = http_config
        .as_object()
        .unwrap()
        .keys()
        .map(|x| x.as_str())
        .collect::<Vec<&str>>()
        .first()
        .unwrap()
        .to_string();

    let hostname_config = http_config.get(hostname).unwrap();
    let redirect_to_address = Arc::new(
        hostname_config
            .get("redirect_to")
            .unwrap()
            .as_str()
            .unwrap(),
    );
    let max_request_size_config = hostname_config
        .get("max_request_size")
        .unwrap()
        .as_str()
        .unwrap();
    let max_request_size: Arc<usize>;

    if max_request_size_config.ends_with("kb") {
        max_request_size = Arc::new(
            max_request_size_config
                .replace("kb", "")
                .parse::<usize>()
                .unwrap()
                * 1024,
        );
    } else if max_request_size_config.ends_with("mb") {
        max_request_size = Arc::new(
            max_request_size_config
                .replace("mb", "")
                .parse::<usize>()
                .unwrap()
                * 1024
                * 1024,
        );
    } else {
        max_request_size =
            Arc::new(max_request_size_config.parse::<usize>().unwrap());
    }

    let mut stream = stream.lock().await;
    let mut reader = BufReader::new(stream.deref_mut());

    let received: Vec<u8> = reader
        .fill_buf()
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to read from stream: {}", e);

            &[0u8; 0]
        })
        .to_vec();

    let received_length = Arc::new(received.len());
    let cloned_received_length = Arc::clone(&received_length);
    // Mark the bytes read as consumed so the buffer will not return them in a subsequent read
    // reader.consume(cloned_received_length);

    if !received.is_empty() {
        let request = String::from_utf8(received.clone()).unwrap_or_else(|e| {
            eprintln!("Failed to convert bytes to string: {}", e);

            String::new()
        });

        let request = extract_request_from_tcp_stream(request.as_str());

        match request {
            Ok(request) => {
                // TODO: Also handle the chunked transfer encoding (content length unknown
                // at start of request, chunked encoding will indicate when the end is reached)

                let content_length = request
                    .headers
                    .into_iter()
                    .find(|(key, _)| key.to_lowercase() == "content-length");

                if content_length.is_some()
                    && content_length.unwrap().1.parse::<usize>().unwrap()
                        > *max_request_size
                {
                    stream
                        .write_all("HTTP/1.1 413 Request Entity Too Large\rContent-Length: 0\r\r".as_bytes())
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to write to stream: {}", e);
                        });

                    return;
                }

                // TODO: Currently the max read amount is 8kb. This should be configurable.
                if cloned_received_length > max_request_size {
                    stream
                        .write_all(
                            b"HTTP/1.1 413 Request Entity Too Large\r\nContent-Length: 63\r\n\r\n<html><body><h1>413 Request Entity Too Large</h1></body></html>"
                        )
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to write to stream: {}", e);
                        });

                    return;
                }

                let addr = &*Arc::clone(&redirect_to_address);
                let connection = TcpStream::connect(addr).await.map_err(|e| {
                    eprintln!(
                        "Failed to connect to {}: {}",
                        redirect_to_address, e
                    );
                });

                match connection {
                    Ok(mut connection) => {
                        connection
                            .write_all(received.as_slice())
                            .await
                            .unwrap_or_else(|e| {
                                eprintln!("Failed to write to stream: {}", e);
                            });

                        let mut reader = BufReader::new(connection);

                        stream
                            .write_all(reader.fill_buf().await.unwrap_or_else(
                                |e| {
                                    eprintln!(
                                        "Failed to read from stream: {}",
                                        e
                                    );

                                    &[0u8; 0]
                                },
                            ))
                            .await
                            .unwrap_or_else(|e| {
                                eprintln!("Failed to write to stream: {}", e);
                            });
                    }
                    Err(_) => {
                        stream
                            .write_all(
                                b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 50\r\n\r\n<html><body><h1>502 Bad Gateway</h1></body></html>",
                            )
                            .await
                            .unwrap_or_else(|e| {
                                eprintln!("Failed to write to stream: {}", e);
                            });
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to extract request: {}", e);
            }
        }
    }
}
