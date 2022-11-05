/// Build args for the cannonball plugin
///
/// The boolean flags should be essentially self explanatory, but `plugin` is the path
/// to the cannonball plugin libcannonball.so and `sock` is a path to the unix socket
/// that the plugin will use to communicate with the client.
pub fn cannonball_args(
    plugin: String,
    branches: bool,
    syscalls: bool,
    pc: bool,
    reads: bool,
    writes: bool,
    instrs: bool,
    sock: String,
) -> Vec<String> {
    let mut args = Vec::new();
    args.push("-plugin".to_string());
    args.push(format!(
        concat!(
            "{},trace_branches={},trace_syscalls={},trace_pc={},trace_reads={},",
            "trace_writes={},trace_instrs={},sock_path={}"
        ),
        plugin,
        if branches { "on" } else { "off" },
        if syscalls { "on" } else { "off" },
        if pc { "on" } else { "off" },
        if reads { "on" } else { "off" },
        if writes { "on" } else { "off" },
        if instrs { "on" } else { "off" },
        sock
    ));
    args
}
