# Cannonball 💣

Cannonball is a producer/consumer framework for QEMU program instrumentation and tracing.

It allows instrumentation of:

* Program counter
* Memory read and write addresses
* Executed instruction opcodes
* Branch executions
* Syscall number/argument/return values

## Building

### Dependencies

You will need `meson`, `ninja`, and `cargo`, as well as the dependencies installed by
running `apt-get build-dep qemu`.

### Compiling

The build system is a bit of a work in progress, but this will build the client library,
the plugin, the example tools and tests, and qemu x86_64.

```sh
meson -Dtarget_list=x86_64 builddir
meson compile -C builddir
```

The plugin will be output to `builddir/libcannonball.so`, and an example tool that dumps
received events to json using the client library will be output to `builddir/cannonball-tools`.



## Running

Running the plugin is done by running:

```sh
qemu-x86_64 -plugin ./builddir/libcannonball.so,help=true -- $(which cat) /etc/shadow # ;)
```

Arguments are passed to `cannonball` as comma separated arg, value pairs separated by a
`=`. The above example shows how to print the help message, which will show the argument
options.

When run, the plugin will wait before execution for the socket passed in `sock_path` to
be opened for listening. Your program should open that unix socket and listen on it for
events. An example listener is provided in
[cannonball-client/test/server_test](cannonball-client/test/server_test/main.rs). The
top item on the roadmap is to make this process a little easier, but for now you can
compile and run the server test with:

```sh
cd cannonball-client
cargo build --release
./target/release/server_test
```

Making the above easier will also make this process less error-prone, but you should
start qemu *before* starting the server_test binary, otherwise both will hang.

## Peeeeerffffff

Cannonball isn't the *fastest* tracer in the west (I believe that title belongs to
[cannoli](https://github.com/MarginResearch/cannoli)), but it aims to be really really
fast!

Cannonball uses a few technologies to get its speed:

* [Tokio](https://tokio.rs) lets us submit event messages from QEMU and have them
  dispatched (also asynchronously) over a Unix socket to a consumer.
* Minimal Instrumentation: enable only what you need to trace and avoid extra callbacks
* Rust FFI: the qemu plugin calls out to rust code as soon as possible, Rust isn't a
  magic bullet for speed, but there are way less footguns to slow you down than in C.