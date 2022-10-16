//! Client test for io_uring socket messaging
use std::time::Duration;

use futures::{sink::SinkExt, FutureExt};
use tokio::{net::UnixStream, select, time::timeout};

use cannonball_client::qemu_event::{QemuEventCodec, QemuEventExec};
use tokio_util::{codec::Framed, sync::CancellationToken};

const SOCK_NAME: &str = "/dev/shm/cannonball.sock";

async fn go(
    framed: &mut Framed<UnixStream, QemuEventCodec>,
) -> Result<(), Box<dyn std::error::Error>> {
    let q = QemuEventExec::new_random();
    const BATCH_SIZE: usize = 64;

    let mut ctr = 0;

    loop {
        framed.feed(q).await?;
        ctr += 1;
        if ctr % BATCH_SIZE == 0 {
            framed.flush().await?;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = UnixStream::connect(SOCK_NAME).await?;
    let mut framed = Framed::new(stream, QemuEventCodec {});
    let token = CancellationToken::new();
    let tok_clone = token.clone();

    let fut = tokio::spawn(async move {
        select! {
            _ = tok_clone.cancelled() => {
                println!("Cancelled");
            }
            _ = go(&mut framed).fuse() => {
                println!("Done");
            }
        }
    });

    let timeout = timeout(Duration::from_secs(10), fut).await;

    if timeout.is_err() {
        println!("Timeout");
        token.cancel();
    }

    Ok(())
}
