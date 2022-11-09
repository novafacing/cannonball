//! Jaivana driver binary
//!
//! This is the main entry point for the Jaivana driver, and puts *everything* together to
//! create an all-in-one binary tracing tool.

use std::path::PathBuf;

use clap::Parser;
use memfd_exec::{MemFdExecutable, Stdio};
use qemu::qemu_x86_64;
use tokio;

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
    #[clap(short, long)]
    pub input_file: Option<PathBuf>,
    /// An output file to write the program's output to. If not set, the program's output will be written to this driver's stdout.
    /// The program to run
    #[clap()]
    pub program: PathBuf,
    /// The arguments to the program
    #[clap(num_args = 1.., last = true)]
    pub args: Vec<String>,
}

#[tokio::main]
async fn main() {}
