//! Run the cannonball plugin and output the trace events to a JSON file.

use clap::Parser;
use futures::stream::StreamExt;
use log::{error, LevelFilter};
// use memfd_exec::Executable;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use simple_logger::SimpleLogger;
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::net::{UnixListener as StdUnixListener, UnixStream as StdUnixStream},
    path::{Path, PathBuf},
    process::exit,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::{
    net::{unix::SocketAddr, UnixListener, UnixStream},
    process::Command,
    time::sleep,
};
use tokio_util::codec::Framed;

use cannonball::args::cannonball_args;
use cannonball::qemu_event::{EventFlags, QemuMsgCodec};
use memfd_exec::{MemFdExecutable, Stdio};
use qemu::qemu_x86_64;

#[derive(Parser, Debug)]
struct Args {
    /// A path to the plugin
    #[clap(short, long)]
    plugin: String,
    /// Log level
    #[clap(short = 'L', long, default_value = "error")]
    log_level: LevelFilter,
    /// Whether to log branches
    #[clap(short, long)]
    branches: bool,
    /// Whether to log syscalls
    #[clap(short, long)]
    syscalls: bool,
    /// Whether to log the pc
    #[clap(short = 'P', long)]
    pc: bool,
    /// Whether to log reads
    #[clap(short, long)]
    reads: bool,
    /// Whether to log writes
    #[clap(short, long)]
    writes: bool,
    /// Whether to log instrs
    #[clap(short, long)]
    instrs: bool,
    /// The program to run
    #[clap()]
    program: PathBuf,
    /// An input file to feed to the program
    #[clap(short = 'I', long)]
    input_file: Option<PathBuf>,
    /// The arguments to the program
    #[clap(num_args = 1.., last = true)]
    args: Vec<String>,
}

async fn handle(stream: StdUnixStream, syscalls: bool) {
    stream.set_nonblocking(true).unwrap();
    let estream = UnixStream::from_std(stream).unwrap();
    let mut framed = Framed::new(estream, QemuMsgCodec {});

    let mut ctr = 0;
    loop {
        if let Some(Ok(event)) = framed.next().await {
            println!("{}", serde_json::to_string(&event).unwrap());
            ctr += 1;
        }
    }
}

// #[tokio::main]
fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

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
    let qemu_bytes = qemu_x86_64();
    let mut qemu = MemFdExecutable::new("qemu-x86_64", qemu_bytes)
        .args(cannonball_args(
            args.plugin,
            args.branches,
            args.syscalls,
            args.pc,
            args.reads,
            args.writes,
            args.instrs,
            sockname.clone(),
        ))
        .arg("--")
        .arg(args.program)
        .args(args.args)
        .stdin(if args.input_file.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start qemu process");

    let mut threads = Vec::new();

    let listener = match StdUnixListener::bind(sockname.clone()) {
        Ok(l) => l,
        Err(e) => {
            error!("Error binding socket: {}", e);
            StdUnixListener::bind(sockname).unwrap()
        }
    };

    eprintln!("Waiting for connection on {:?}", listener.local_addr());

    let listener_thread = thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    eprintln!("Got connection from {:?}", stream.peer_addr());
                    rt.spawn(handle(stream, args.syscalls));
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }
    });
    threads.push(listener_thread);

    if args.input_file.is_some() {
        let mut stdin = qemu.stdin.take().unwrap();
        let mut input_file = File::open(args.input_file.unwrap()).unwrap();
        let mut buf = Vec::new();
        input_file.read_to_end(&mut buf).unwrap();
        let writer_thread = thread::spawn(move || {
            stdin.write_all(&buf).unwrap();
        });
        threads.push(writer_thread);
    }

    let status = qemu.wait().unwrap();
    eprintln!("Qemu exited with status: {}", status.code().unwrap());
    // wait on the threads
    for thread in threads {
        thread.join().unwrap();
    }
}
