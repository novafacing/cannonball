mod events;

use clap::Parser;
use memfd_exec::{MemFdExecutable, Stdio};
use qemu::qemu_x86_64;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_cbor::Deserializer;
use std::{
    error::Error,
    fs::File,
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
};
use tokio::{fs::write, io::AsyncWriteExt, join, spawn, task::spawn_blocking};

use events::Event;

#[derive(Parser, Debug)]
/// Trace a program with the Jaivana QEMU plugin
struct Args {
    /// Whether to log instructions. If set, all instructions will be logged.
    #[clap(short, long)]
    pub insns: bool,
    /// Whether to log branches. If `insns` is not set, only branch instructions will be logged.
    #[clap(short, long)]
    pub branches: bool,
    /// Whether to log opcodes. If not set, only the instruction address will be log
    #[clap(short, long)]
    pub opcodes: bool,
    /// Whether to log syscalls. If set, all syscalls will be logged.
    #[clap(short, long)]
    pub syscalls: bool,
    /// Whether to log memory accesses. If set, memory accesses for already instrumented instructions will be logged.
    #[clap(short, long)]
    pub mem: bool,
    /// An input file to feed to the program. If not set, the program will take input via this driver's stdin.
    #[clap(short = 'I', long)]
    pub input_file: Option<PathBuf>,
    /// An output file to write the program's output to. If not set, the program's output will be written to this driver's stdout.
    #[clap(short = 'O', long)]
    pub output_file: Option<PathBuf>,
    /// The program to run
    #[clap()]
    pub program: PathBuf,
    /// The arguments to the program
    #[clap(num_args = 1.., last = true)]
    pub args: Vec<String>,
}

async fn run_qemu(
    input_data: Option<Vec<u8>>,
    args: Vec<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let qemu = qemu_x86_64();
    let mut exe = MemFdExecutable::new("qemu-x86_64", qemu)
        .args(args)
        .stdin(if input_data.is_none() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn QEMU");

    let mut stdin: Option<_> = if input_data.is_some() {
        Some(exe.stdin.take().expect("Failed to get stdin"))
    } else {
        None
    };

    let writer = spawn_blocking(move || match stdin {
        Some(ref mut stdin) => {
            stdin.write_all(&input_data.unwrap()).unwrap();
        }
        None => {}
    });

    let mut stdout = exe.stdout.take().expect("Failed to get stdout");
    let mut stderr = exe.stderr.take().expect("Failed to get stderr");

    let reader = spawn_blocking(move || {
        let mut output = Vec::new();
        stdout.read_to_end(&mut output).unwrap();
        println!("Output: {:?}", output);
    });

    let ereader = spawn_blocking(move || {
        let mut output = Vec::new();
        stderr.read_to_end(&mut output).unwrap();
        println!("Error: {:?}", output);
    });

    let waiter = spawn_blocking(move || {
        exe.wait().expect("Failed to wait for QEMU");
    });

    let (writeres, readeres, ereaderes, waiteres) = join!(writer, reader, ereader, waiter);

    writeres?;
    readeres?;
    ereaderes?;
    waiteres?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let sockid = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>();
    let sockpath = PathBuf::from(format!("/tmp/qemu-{}.sock", sockid));

    let program_path = args
        .program
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let input_data = match args.input_file {
        Some(path) => Some(
            tokio::fs::read(path)
                .await
                .expect("Failed to read input file"),
        ),
        None => None,
    };

    #[cfg(debug_assertions)]
    let plugin = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/debug/libmons_meg.so"
    ));

    #[cfg(not(debug_assertions))]
    let plugin = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/release/libmons_meg.so"
    ));

    let pluginid = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>();
    let pluginpath = PathBuf::from(format!("/tmp/qemu-{}.so", pluginid));
    write(&pluginpath, plugin).await.unwrap();
    let plugin_args = format!(
        "{},log_pc={},log_opcode={},log_branch={},log_mem={},log_syscall={},socket_path={}",
        pluginpath.to_str().unwrap(),
        args.insns,
        args.opcodes,
        args.branches,
        args.syscalls,
        args.mem,
        sockpath.to_str().unwrap()
    )
    .to_string();

    let mut qemu_args = vec!["-plugin".to_string(), plugin_args];
    qemu_args.push("--".to_string());
    qemu_args.push(program_path);
    qemu_args.extend(args.args);

    let listen_sock = UnixListener::bind(&sockpath).unwrap();

    let mut outfile_stream = match args.output_file {
        Some(path) => {
            let file = File::create(path).expect("Failed to create output file");
            Some(file)
        }
        None => None,
    };

    let qemu_task = spawn(async move { run_qemu(input_data, qemu_args).await });
    // Spawn a task that reads from the socket and decodes the cbor encoded data
    let socket_task = spawn_blocking(move || {
        let (mut stream, _) = listen_sock.accept().unwrap();
        let it = Deserializer::from_reader(&mut stream).into_iter::<Event>();
        for event in it {
            match outfile_stream {
                Some(ref mut file) => {
                    let event = event.unwrap();
                    file.write_all(format!("{:?}\n", event).as_bytes())
                        .expect("Failed to write to output file");
                }
                None => {
                    println!("{:?}", event.unwrap());
                }
            }
        }
    });

    let (qemu_res, socket_res) = join!(qemu_task, socket_task);
    qemu_res.unwrap().unwrap();
    socket_res.unwrap();
}
