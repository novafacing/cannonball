//! Server test for io_uring socket messaging

use std::{fs::remove_file, path::Path};

use cannonball_client::qemu_event::{QemuEventCodec, EventFlags};
use futures::stream::StreamExt;
use tokio::net::{unix::SocketAddr, UnixListener, UnixStream};
use tokio_util::codec::Framed;

const SOCK_NAME: &str = "/dev/shm/cannonball.sock";

async fn handle(_addr: SocketAddr, stream: UnixStream) {
    let mut framed = Framed::new(stream, QemuEventCodec {});

    let mut ctr = 0;
    loop {
        if let Some(Ok(event)) = framed.next().await {
            ctr += 1;
            println!("Received {} events", ctr);
            println!("Received event: {:?}", event);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If the socket already exists, delete it
    if Path::new(SOCK_NAME).exists() {
        remove_file(SOCK_NAME)?;
    }

    let listener = match UnixListener::bind(SOCK_NAME) {
        Ok(l) => l,
        Err(e) => {
            println!("Error binding socket: {}", e);
            UnixListener::bind(SOCK_NAME).unwrap()
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                tokio::spawn(async move {
                    handle(addr, stream).await;
                });
            }
            Err(e) => {
                println!("Error accepting connection: {}", e);
            }
        }
    }
}
