//! Jaivana driver binary
//!
//! This is the main entry point for the Jaivana driver, and puts *everything* together to
//! create an all-in-one binary tracing tool.

use clap::Parser;
use memfd_exec::{MemFdExecutable, Stdio};
use qemu::qemu_x86_64;

use std::{
    env::temp_dir,
    fs::{read, write},
    io::{Read, Write},
    path::PathBuf,
    thread::spawn,
};

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

fn main() {
    let args = Args::parse();

    #[cfg(debug_assertions)]
    let plugin = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/debug/libjaivana.so"
    ));

    #[cfg(not(debug_assertions))]
    let plugin = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/release/libjaivana.so"
    ));

    let plugin_args = format!(
        "log_pc={},log_branch={},log_opcode={},log_syscall={},log_mem={}",
        args.insns, args.branches, args.opcodes, args.syscalls, args.mem
    );

    let qemu = qemu_x86_64();

    // Write the plugin to a temporary file
    let plugin_path = temp_dir().join("libjaivana.so");

    let program_path = args
        .program
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();

    write(&plugin_path, plugin).unwrap();

    let mut exe = MemFdExecutable::new("qemu-x86_64", qemu)
        .arg("-plugin")
        .arg(format!(
            "{},{}",
            plugin_path.canonicalize().unwrap().to_string_lossy(),
            plugin_args
        ))
        .arg("--")
        .arg(program_path)
        .args(args.args)
        .stdin(if args.input_file.is_some() {
            Stdio::piped()
        } else {
            Stdio::Inherit
        })
        .stdout(if args.output_file.is_some() {
            Stdio::piped()
        } else {
            Stdio::Inherit
        })
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn QEMU");

    if let Some(input_file) = args.input_file {
        let mut stdin = exe.stdin.take().expect("Failed to get stdin");
        let input = read(input_file).expect("Failed to read input file");
        spawn(move || {
            stdin.write(&input).expect("Failed to write input");
        });
    }

    if let Some(output_file) = args.output_file {
        let mut stdout = exe.stdout.take().expect("Failed to get stdout");
        let mut output = Vec::new();
        spawn(move || {
            stdout
                .read_to_end(&mut output)
                .expect("Failed to read output");
            write(output_file, output).expect("Failed to write output");
        });
    }

    exe.wait().expect("Failed to wait for QEMU");
}
