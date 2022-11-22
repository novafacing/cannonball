# Jaivana

This is about the simplest possible usage of Cannonball. We create a QEMU plugin that 
traces various events and prints out the JSON-encoded events to stdout.

## Usage

```
$ ./target/debug/jaivana -h
Trace a program with the Jaivana QEMU plugin

Usage: jaivana [OPTIONS] <PROGRAM> [-- <ARGS>...]

Arguments:
  <PROGRAM>  The program to run
  [ARGS]...  The arguments to the program

Options:
  -i, --insns                      Whether to log instructions. If set, all instructions will be logged
  -b, --branches                   Whether to log branches. If `insns` is not set, only branch instructions will be logged
  -o, --opcodes                    Whether to log opcodes. If not set, only the instruction address will be log
  -s, --syscalls                   Whether to log syscalls. If set, all syscalls will be logged
  -m, --mem                        Whether to log memory accesses. If set, memory accesses for already instrumented instructions will be logged
  -I, --input-file <INPUT_FILE>    An input file to feed to the program. If not set, the program will take input via this driver's stdin
  -O, --output-file <OUTPUT_FILE>  An output file to write the program's output to. If not set, the program's output will be written to this driver's stdout
  -h, --help                       Print help information
```