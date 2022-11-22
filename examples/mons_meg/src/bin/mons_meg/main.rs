mod events;

use std::{
    error::Error,
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
};

use events::Event;
use memfd_exec::{MemFdExecutable, Stdio};
use qemu::qemu_x86_64;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_cbor::{from_reader, from_slice, Deserializer, StreamDeserializer};
use tokio::{fs::write, join, spawn, task::spawn_blocking};

async fn run_qemu(args: Vec<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let qemu = qemu_x86_64();
    let mut exe = MemFdExecutable::new("qemu-x86_64", qemu)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn QEMU");

    let mut stdin = exe.stdin.take().expect("Failed to get stdin");
    let mut stdout = exe.stdout.take().expect("Failed to get stdout");
    let mut stderr = exe.stderr.take().expect("Failed to get stderr");

    let writer = spawn_blocking(move || {
        stdin.write_all(b"Hello, world!").unwrap();
    });

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
    let sockid = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>();
    let sockpath = PathBuf::from(format!("/tmp/qemu-{}.sock", sockid));

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
        true,
        true,
        true,
        false,
        false,
        sockpath.to_str().unwrap()
    )
    .to_string();

    let mut qemu_args = vec!["-plugin".to_string(), plugin_args];
    qemu_args.push("--".to_string());
    qemu_args.push("/bin/cat".to_string());

    let listen_sock = UnixListener::bind(&sockpath).unwrap();

    let qemu_task = spawn(async move { run_qemu(qemu_args).await });
    // Spawn a task that reads from the socket and decodes the cbor encoded data
    let socket_task = spawn_blocking(move || {
        let (mut stream, _) = listen_sock.accept().unwrap();
        let it = Deserializer::from_reader(&mut stream).into_iter::<Event>();
        for event in it {
            println!("{:?}", event);
        }
    });

    let (qemu_res, socket_res) = join!(qemu_task, socket_task);
    qemu_res.unwrap().unwrap();
    socket_res.unwrap();
}
