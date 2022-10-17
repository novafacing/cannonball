//! Run the cannonball plugin and output the trace events to a JSON file.

use clap::Parser;
use futures::stream::StreamExt;
use log::LevelFilter;
use memfd_exec::Executable;
use simple_logger::SimpleLogger;
use std::fs::Path;
use tokio::net::{unix::SocketAddr, UnixListener, UnixStream};
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
    #[clap(multiple_values = true, last = true)]
    args: Vec<String>,
}

#[tokio::main]
fn main() {
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

    let listener = match UnixListener::bind(sockname) {
        Ok(l) => l,
        Err(e) => {
            error!("Error binding socket: {}", e);
            UnixListener::bind(sockname).unwrap()
        }
    };
}
