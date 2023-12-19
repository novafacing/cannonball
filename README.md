# Cannonball 💣

## DEPRECATION NOTICE

This project has been superceded by the
[qemu-plugin](https://github.com/novafacing/qemu-rs/tree/main/qemu) bindings
provided by the [qemu-rs](https://github.com/novafacing/qemu-rs) project.

Cannonball is a framework for building QEMU plugins in Rust! Anything you can do in
a QEMU TCG plugin in C, you can do with `cannonball`.

Write plugins that run with minimal overhead and as much functionality as you can dream
of!

## Examples

There are a couple examples provided here!

* [`jaivana`](examples/jaivana/README.md) A simple tracer that logs a configurable set of events to a file or stdout.
* [`mons meg`](examples/mons_meg/README.md) A tracer that logs the same events as Jaivana, but uses Tokio to run the trace in an async environment, with communication
  with the host over a UNIX socket instead of anonymous pipes.

Take a look at them, they are the best way to learn how to use this framework.

## Installation

Just add this to your `Cargo.toml`:

```toml
cannonball = "0.2.4"
```

## Example

Here's a quick recording of the [Jaivana](./examples/jaivana) example plugin and driver!

[![asciicast](https://asciinema.org/a/a1y3n6CqJEq3Yk7SDwJVrTrWi.svg)](https://asciinema.org/a/a1y3n6CqJEq3Yk7SDwJVrTrWi)
