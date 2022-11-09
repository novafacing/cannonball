//! Jaivana event tracing plugin
//!
//! This is a useful plugin, but also serves as an exmaple of what
//! the `cannonball` framework can do. For now, the plugin only fully
//! supports QEMU user mode, but it would only take a little work to
//! support system mode as well.
//!
//! Jaivana can log the following events:
//!
//! * Instruction execution:
//!     * The program counter (PC)
//!     * The instruction opcode
//!     * Whether the instruction terminates a basic block
//!     * Memory reads and writes (read/write vaddr)
//! * System calls:
//!     * Syscall number
//!     * Syscall arguments
//!     * Syscall return value

mod events;

use cannonball::{
    api::{
        qemu_info_t, qemu_plugin_insn_data, qemu_plugin_insn_size, qemu_plugin_insn_vaddr,
        qemu_plugin_mem_is_big_endian, qemu_plugin_mem_is_sign_extended, qemu_plugin_mem_is_store,
        qemu_plugin_mem_size_shift, qemu_plugin_meminfo_t, qemu_plugin_tb, qemu_plugin_tb_get_insn,
        qemu_plugin_tb_n_insns,
    },
    args::{Args, QEMUArg},
    callbacks::{
        RegisterInsnExec, SetupCallback, SetupCallbackType, StaticCallbackType,
        VCPUInsnExecCallback, VCPUMemCallback, VCPUSyscallCallback, VCPUSyscallRetCallback,
        VCPUTBTransCallback,
    },
};
use inventory::submit;
use lazy_static::lazy_static;
use libc::c_void;
use once_cell::sync::Lazy;

use events::{InsnEvent, MemEvent, SyscallEvent};
use serde_json::to_string;

use std::{collections::HashMap, ffi::CStr, num::Wrapping, slice::from_raw_parts, sync::Mutex};

#[derive(Debug)]
struct Context {
    // Info obtained from qemu info on startup
    // Target name (usually the binary name or path)
    pub target_name: Option<String>,
    // Minimum, current plugin API version
    pub version: Option<(i32, i32)>,
    // Is this a system emulation?
    pub system_emulation: Option<bool>,
    // Initial, maximum VCPU count
    pub vcpus: Option<(i32, i32)>,

    // Original arguments to the plugin
    pub args: Option<Args>,

    // Settings enabling/disabling logging of events
    pub log_pc: bool,
    pub log_opcode: bool,
    pub log_branch: bool,
    pub log_mem: bool,
    pub log_syscall: bool,

    // Temporary storage for the last syscall executed on each (plugin id, vcpu) pair
    // stores the syscall arguments and number until the syscall returns, then the return
    // value can be associated and the event can be dispatched and removed from this map
    pub syscalls: HashMap<(u64, u32), SyscallEvent>,
    // Sequential ephemeral key for indexing temporary instruction store
    pub ikey: Wrapping<u64>,
    pub klimit: Wrapping<u64>,
    // Temporary store for instructions, indexed by ephemeral sequential key `ikey`
    // stores an instruction from the time it is translated until it is either executed
    // or a memory access is made, at which point the instruction is dispatched and removed
    pub insns: HashMap<u64, InsnEvent>,
}

impl Context {
    /// Instantiate a new trace context
    ///
    /// # Arguments
    ///
    /// * `target_name` - The name of the target binary
    /// * `version` - The minimum and current plugin API version
    /// * `system_emulation` - Whether this is a system emulation
    /// * `vcpus` - The initial and maximum VCPU count
    /// * `args` - The original arguments to the plugin
    /// * `log_pc` - Whether to log the program counter
    /// * `log_opcode` - Whether to log the instruction opcode
    /// * `log_branch` - Whether to log whether the instruction terminates a basic block
    /// * `log_mem` - Whether to log memory accesses
    /// * `log_syscall` - Whether to log system calls
    /// * `syscalls` - The temporary storage for the last syscall executed on each (plugin id, vcpu) pair
    /// * `ikey` - The sequential ephemeral key for indexing temporary instruction store
    /// * `insns` - The temporary store for instructions, indexed by ephemeral sequential key `ikey`
    pub fn new() -> Self {
        Self {
            target_name: None,
            version: None,
            system_emulation: None,
            vcpus: None,
            args: None,
            log_pc: false,
            log_opcode: false,
            log_branch: false,
            log_mem: false,
            log_syscall: false,
            syscalls: HashMap::new(),
            ikey: Wrapping(0),
            klimit: Wrapping(1024),
            insns: HashMap::new(),
        }
    }

    /// Return an incrementing sequential key for indexing temporary instruction store and reap
    /// old entries in case something goes wrong and a callback is not triggered for them
    pub fn ikey(&mut self) -> u64 {
        let key = self.ikey;
        let reap = key - self.klimit;
        self.insns.remove(&reap.0);
        self.ikey += Wrapping(1);
        key.0
    }
}

lazy_static! {
    /// The global context for the tracing plugin
    static ref CONTEXT: Mutex<Context> = Mutex::new(Context::new());
}

#[derive(Clone)]
// `*mut c_void` is not `Send + Sync` so we need to use a newtype to wrap it. The `From` and
// `Into` implementations are for convenience, we could just as easily `as` it around in
// the code.
struct ExecKey(*mut c_void);

unsafe impl Send for ExecKey {}
unsafe impl Sync for ExecKey {}

impl ExecKey {
    fn new(v: u64) -> Self {
        Self(v as *mut c_void)
    }
}

impl Into<*mut c_void> for ExecKey {
    fn into(self) -> *mut c_void {
        self.0
    }
}

impl From<*mut c_void> for ExecKey {
    fn from(v: *mut c_void) -> Self {
        Self(v)
    }
}

impl Into<u64> for ExecKey {
    fn into(self) -> u64 {
        self.0 as u64
    }
}

/// Called on plugin load with the arguments passed to the plugin on the command
/// line. We use this function to initialize our global context with the information
/// QEMU provides us about the target, including the name, whether we are running in
/// system mode, and the number of VCPUs.
extern "C" fn setup(info: *const qemu_info_t, args: &Args) {
    let mut jv = CONTEXT.lock().unwrap();
    unsafe {
        let info = &*info;
        jv.target_name = Some(
            CStr::from_ptr(info.target_name)
                .to_string_lossy()
                .to_string(),
        );
        jv.version = Some((info.version.cur, info.version.min));
        jv.system_emulation = Some(info.system_emulation);
        jv.vcpus = Some((
            info.__bindgen_anon_1.system.smp_vcpus,
            info.__bindgen_anon_1.system.max_vcpus,
        ));
    }

    jv.args = Some(args.clone());

    // We can use the args to selectively enable/disable logging of events
    if let Some(QEMUArg::Bool(log_pc)) = args.args.get("log_pc") {
        jv.log_pc = *log_pc;
    }

    if let Some(QEMUArg::Bool(log_opcode)) = args.args.get("log_opcode") {
        jv.log_opcode = *log_opcode;
    }

    if let Some(QEMUArg::Bool(log_branch)) = args.args.get("log_branch") {
        jv.log_branch = *log_branch;
    }

    if let Some(QEMUArg::Bool(log_mem)) = args.args.get("log_mem") {
        jv.log_mem = *log_mem;
    }

    if let Some(QEMUArg::Bool(log_syscall)) = args.args.get("log_syscall") {
        jv.log_syscall = *log_syscall;
    }
}

submit! {
    // Register the `SetupCallback` function to run during plugin setup
    static scb: Lazy<SetupCallback> = Lazy::new(|| {
        SetupCallback::new(|info, args| {
            setup(info, args);
        })
    });
    SetupCallbackType::Setup(&scb)
}

/// Called on execution of each instruction after registration in `on_tb_trans`. This
/// function just logs the instruction at the time it is executed (instead of at the time
/// it is translated, which does not necessarily happen in execution order)
unsafe extern "C" fn on_insn_exec(vcpu_idx: u32, data: *mut c_void) {
    let mut jv = CONTEXT.lock().unwrap();
    // Since `ExecKey` is a newtype we can just cast it back. If you get really fancy, you can
    // use a `Box::into_raw(Box::new(T))` pattern to pass around a full object, but it is easier
    // for the sake of example to store it globally. The callback types do support more
    // complex use cases though.
    let ekey: ExecKey = data.into();
    let key: u64 = ekey.into();

    if let Some(insn_evt) = jv.insns.get(&key) {
        let mut insn_evt = insn_evt.clone();
        insn_evt.vcpu_idx = Some(vcpu_idx);
        let insn_evt = to_string(&insn_evt).unwrap();
        println!("{}", insn_evt);

        jv.insns.remove(&key);
    }
}

/// Called on memory access by an instruction, but not necessarily before or after the instruction
/// executes. Therefore, we use a second duplicate entry of the original isntruction to back-
/// correlate memory accesses with executions, but we don't know which comes first.
unsafe extern "C" fn on_mem_access(
    vcpu_index: u32,
    info: qemu_plugin_meminfo_t,
    vaddr: u64,
    data: *mut c_void,
) {
    let mut jv = CONTEXT.lock().unwrap();
    let ekey: ExecKey = data.into();
    let key: u64 = ekey.into();

    if let Some(insn_evt) = jv.insns.get(&key) {
        let mut insn_evt = insn_evt.clone();
        insn_evt.vcpu_idx = Some(vcpu_index);

        let is_sext = qemu_plugin_mem_is_sign_extended(info);
        let is_be = qemu_plugin_mem_is_big_endian(info);
        let is_store = qemu_plugin_mem_is_store(info);
        let size_shift = qemu_plugin_mem_size_shift(info);

        let mem_evt = MemEvent::new(
            vaddr,
            is_sext,
            is_be,
            is_store,
            size_shift,
            insn_evt.clone(),
        );

        let json = to_string(&mem_evt).unwrap();
        println!("{}", json);

        jv.insns.remove(&key);
    }
}

/// Called on translation of a new translation block. We use this function to register additional
/// callbacks for execution and memory access. We also use this function to populate
/// information about the instructions, depending on what logging is enabled by the arguments
unsafe extern "C" fn on_tb_trans(_id: u64, tb: *mut qemu_plugin_tb) {
    let mut jv = CONTEXT.lock().unwrap();

    let n_isns = qemu_plugin_tb_n_insns(tb);
    let first_insn = if jv.log_pc || jv.log_mem {
        0
    } else if jv.log_branch {
        n_isns - 1
    } else {
        // TODO: We can probably eliminate this overhead but for example's sake
        // this is probably fine. Skip the whole TB if we aren't logging anything
        n_isns
    };

    for insn_idx in first_insn..n_isns {
        let branch = insn_idx == n_isns - 1;
        let insn = qemu_plugin_tb_get_insn(tb, insn_idx);
        let vaddr = qemu_plugin_insn_vaddr(insn);

        let mut evt = InsnEvent::new(None, vaddr, None, branch);

        if jv.log_opcode {
            let opcode_len = qemu_plugin_insn_size(insn);
            let raw_opcode = qemu_plugin_insn_data(insn);
            // reinterpret the raw opcode as a slice of bytes
            let opcode: Vec<u8> = from_raw_parts(raw_opcode as *const u8, opcode_len as usize)
                .iter()
                .map(|x| *x)
                .collect();

            evt.opcode = Some(opcode);
        }

        let exec_key = *&jv.ikey();
        jv.insns.insert(exec_key, evt.clone());

        let exec_cb = VCPUInsnExecCallback::new(on_insn_exec, ExecKey::new(exec_key));
        exec_cb.register(insn);

        if jv.log_mem {
            let mem_key = *&jv.ikey();
            jv.insns.insert(mem_key, evt.clone());

            let mem_cb = VCPUMemCallback::new(on_mem_access, ExecKey::new(mem_key));
            mem_cb.register(insn);
        }
    }
}

submit! {
    // VCPUTBTransCallback is also a static callback that must be registered in
    // `qemu_plugin_install`, so we need to submit it as an inventory item.
    static tbcb: Lazy<VCPUTBTransCallback> = Lazy::new(|| {
        VCPUTBTransCallback::new(on_tb_trans)
    });
    StaticCallbackType::VCPUTBTrans(&tbcb)
}

/// Called on each system call entry. We use this function to populate the arguments and
/// number of the syscall, and then we store it until we get an event returning from the system
/// call so we can populate the return value.
unsafe extern "C" fn on_syscall(
    id: u64,
    vcpu_idx: u32,
    num: i64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
    arg7: u64,
) {
    let mut jv = CONTEXT.lock().unwrap();

    if jv.log_syscall {
        let args = vec![arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7];
        let syscall = SyscallEvent::new(num, None, args);
        jv.syscalls.insert((id, vcpu_idx), syscall);
    }
}

submit! {
    // VCPUSyscallcallback is also a static callback type, so we register it at
    // installation time
    static syscb: Lazy<VCPUSyscallCallback> = Lazy::new(|| {
        VCPUSyscallCallback::new(on_syscall)
    });
    StaticCallbackType::VCPUSyscall(&syscb)
}

/// Called on each system call exit. We use this function to populate the return value of the
/// system call, and then we print the syscall event.
unsafe extern "C" fn on_syscall_ret(id: u64, vcpu_idx: u32, _num: i64, rv: i64) {
    let mut jv = CONTEXT.lock().unwrap();

    if jv.log_syscall {
        let mut syscall = jv.syscalls.remove(&(id, vcpu_idx)).unwrap();
        syscall.rv = Some(rv);
        println!("{}", to_string(&syscall).unwrap());
    }
}

submit! {
    static sysretcb: Lazy<VCPUSyscallRetCallback> = Lazy::new(|| {
        VCPUSyscallRetCallback::new(on_syscall_ret)
    });
    StaticCallbackType::VCPUSyscallRet(&sysretcb)
}
