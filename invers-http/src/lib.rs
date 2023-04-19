use std::future::Future;
use std::panic;
use std::pin::Pin;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

pub mod utils;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Sync + Send + 'static>>;
type Handler = Box<fn(Arc<Mutex<TcpStream>>) -> BoxFuture>;

pub struct HttpServer {
    listener: TcpListener,
    handlers: Vec<Handler>,
}

impl HttpServer {
    pub async fn new(address: &str) -> HttpServer {
        let maybe_listener = TcpListener::bind(address).await;

        if maybe_listener.is_err() {
            handle_server_start_error(address.to_string());
        }

        Self {
            listener: maybe_listener.unwrap(),
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Handler) -> &mut Self {
        self.handlers.push(handler);

        self
    }

    pub async fn run(&self) {
        loop {
            let tcp_stream_result = self.listener.accept().await;

            match tcp_stream_result {
                Ok((tcp_stream, _)) => {
                    let mutex_stream = Arc::new(Mutex::new(tcp_stream));

                    for handler in self.handlers.iter() {
                        // Not cloning the actual mutex_stream here, but instead we just have
                        // multiple pointers to the same mutex.
                        tokio::spawn(handler(Arc::clone(&mutex_stream)));
                    }
                }
                Err(_) => {
                    eprintln!("Failed to accept connection");
                }
            }
        }
    }
}

fn handle_server_start_error(address: String) {
    panic::set_hook(Box::new(move |why| {
        let message = why.to_string();
        if message.contains("AddrInUse") {
            eprintln!("Failed to bind {}: Address already in use", address);
        } else {
            eprintln!("Failed to start server on {}. Reason unknown", address);
        }

        println!("Shutting down invers");

        std::process::exit(1);
    }));
}
