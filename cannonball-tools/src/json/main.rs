//! Run the cannonball plugin and output the trace events to a JSON file.

use clap::Parser;
use futures::stream::StreamExt;
use log::{error, LevelFilter};
// use memfd_exec::Executable;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use simple_logger::SimpleLogger;
use std::{
    os::unix::net::{UnixListener as StdUnixListener, UnixStream as StdUnixStream},
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

use cannonball_client::qemu_event::{EventFlags, QemuEventCodec};

#[derive(Parser, Debug)]
struct Args {
    // A path to a qemu executable. If not provided and the tool was compiled with
    // qemu built-in, the built-in qemu will be used. If not provided and the tool
    // was not compiled with qemu built-in, the tool will yell at you :)
    #[clap(short, long)]
    qemu: Option<String>,
    // A path to the plugin
    #[clap(short, long)]
    plugin: String,
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

async fn handle(stream: StdUnixStream) {
    stream.set_nonblocking(true).unwrap();
    let estream = UnixStream::from_std(stream).unwrap();
    let mut framed = Framed::new(estream, QemuEventCodec {});

    let mut ctr = 0;
    loop {
        if let Some(Ok(event)) = framed.next().await {
            ctr += 1;
            println!("{}", serde_json::to_string(&event).unwrap());
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

    let sockid: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    // Sock can be in /tmp, not any slower than /dev/shm
    let sockname = format!("/dev/shm/{}.sock", sockid);
    let sockpath = Path::new(&sockname);

    if sockpath.exists() {
        error!("Socket already exists: {}", sockname);
        return;
    }

    tokio::spawn({
        let sname = sockname.clone();
        async move {
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
        .arg(format!("{},trace_branches=true,trace_syscalls=true,trace_pc=true,trace_reads=true,trace_writes=true,trace_instrs=true,sock_path={}", args.plugin, sname))
        .arg("--")
        .arg(args.program)
        .args(args.args)
        .spawn().expect("QEMU failed to start")
        .wait().await.expect("QEMU failed to run");
        }
    });

    let listener = match StdUnixListener::bind(sockname.clone()) {
        Ok(l) => l,
        Err(e) => {
            error!("Error binding socket: {}", e);
            StdUnixListener::bind(sockname).unwrap()
        }
    };

    eprintln!("Waiting for connection on {:?}", listener.local_addr());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                eprintln!("Got connection from {:?}", stream.peer_addr());
                tokio::spawn(handle(stream));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}
