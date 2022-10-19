//! Run the cannonball plugin and output the trace events to a JSON file.

use clap::Parser;
use futures::stream::StreamExt;
use log::{error, LevelFilter};
// use memfd_exec::Executable;
use simple_logger::SimpleLogger;
use std::{
    path::{Path, PathBuf},
    process::exit,
    time::Duration,
};
use tokio::{
    net::{unix::SocketAddr, UnixListener, UnixStream},
    process::Command,
    time::sleep,
};
use tokio_util::codec::Framed;
use uuid::Uuid;

use cannonball_client::qemu_event::{EventFlags, QemuEventCodec};

#[derive(Parser, Debug)]
struct Args {
    // A path to a qemu executable. If not provided and the tool was compiled with
    // qemu built-in, the built-in qemu will be used. If not provided and the tool
    // was not compiled with qemu built-in, the tool will yell at you :)
    #[clap(short, long)]
    qemu: Option<String>,
    /// Log level
    #[clap(short = 'L', long, default_value = "error")]
    log_level: LevelFilter,
    // The program to run
    #[clap()]
    program: PathBuf,
    // The arguments to the program
    #[clap(num_args = 1.., last = true)]
    args: Vec<String>,
}

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
async fn main() {
    let args = Args::parse();
    SimpleLogger::new()
        .with_level(args.log_level)
        .init()
        .unwrap();

    let sockid = Uuid::new_v4().to_string();
    // Sock can be in /tmp, not any slower than /dev/shm
    let sockname = format!("/tmp/{}", sockid);
    let sockpath = Path::new(&sockname);

    if sockpath.exists() {
        error!("Socket already exists: {}", sockname);
        return;
    }

    tokio::spawn(async move {
        Command::new(args.qemu.unwrap_or_else(|| {
            if cfg!(feature = "monolithic") {
                // TODO: This isn't working yet though!
                "qemu".to_string()
            } else {
                error!("No qemu executable provided");
                exit(1);
            }
        }))
        .arg("-plugin")
        .arg("../builddir/libcannonball.so,trace_branches=true,trace_syscalls=true,trace_pc=true,trace_reads=true,trace_writes=true,trace_instrs=true")
        .arg("--")
        .arg(args.program)
        .args(args.args)
        .spawn().expect("QEMU failed to start")
        .wait().await.expect("QEMU failed to run");
    });

    sleep(Duration::from_secs(1)).await;

    let listener = match UnixListener::bind(sockname.clone()) {
        Ok(l) => l,
        Err(e) => {
            error!("Error binding socket: {}", e);
            UnixListener::bind(sockname).unwrap()
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
                error!("Error accepting connection: {}", e);
            }
        }
    }
}
