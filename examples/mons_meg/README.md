# Mons Meg

This is an example of using Cannonball to trace in an async environment using the Tokio
executor. We define some events an use a plugin almost identical to `jaivana` to trace
the events, but instead of printing them out we write them as CBOR-encoded bytes to a
UNIX socket from the plugin.

The host driver program uses memfd-exec to run a QEMU instance with the plugin and reads
and deserializes the event data from the socket and prints it out.

## Usage

```
$ ./target/debug/mons_meg -h
Trace a program with the Jaivana QEMU plugin

Usage: mons_meg [OPTIONS] <PROGRAM> [-- <ARGS>...]

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