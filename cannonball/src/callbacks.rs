//! Callback types
//!
//! The following callbacks may only be registered through `inventory`, because they must be
//! called during `qemu_plugin_install` which is called by QEMU when the plugin is loaded.
//!
//! * `vcpu_init`
//! * `vcpu_exit`
//! * `vcpu_idle`
//! * `vcpu_resume`
//! * `vcpu_tb_trans`
//! * `vcpu_syscall`
//! * `vcpu_syscall_ret`
//! * `atexit`
//! * `flush`
//!
//! These can be registered statically like so. The `Lazy` bit is a little fucked, sorry about
//! that. If you have a nicer way to do this, please let me know @novafacing everywhere fine
//! posts and interactions are sold.
//!
//! ```
//! // Example of a static callback registration for a callback that will be triggered on
//! // plugin registration.
//!
//! use inventory;
//! use once_cell::sync::Lazy;
//! use cannonball::callbacks::{Callback, StaticCallbackType, VCPUTBTransCallback};
//!
//! extern "C" fn testfn(id: u64, tb: *mut qemu_plugin_tb) {
//!     println!("Hello from testfn! We are translating a TB!");
//! }
//!
//! inventory::submit! {
//!     static tcb: Lazy<VCPUTBTransCallback> = Lazy::new(|| { VCPUTBTransCallback::new(testfn) });
//!     StaticCallbackType::VCPUTBTrans(&tcb)
//! }
//! ```
//!
//! There is also a non-QEMU callback used for setup. `SetupCallback` instances can be registered
//! and will be called before QEMU runs. Any global state initialization can be done there.
//!
//! ```
//! // Example of a setup callback registration
//! use inventory;
//! use once_cell::sync::Lazy;
//! use cannonball::callbacks::{SetupCallback, SetupCallbackType};
//!
//! inventory::submit! {
//!     static scb: Lazy<SetupCallback> = Lazy::new(|| {
//!         SetupCallback::new(|info, args| {
//!             println!("setup callback");
//!             println!("info: {:?}", info);
//!             println!("args: {:?}", args);
//!         })
//!     });
//!     SetupCallbackType::Setup(&scb)
//! }
//! ```

use libc::c_void;
use once_cell::sync::Lazy;

use crate::{
    api::{
        qemu_info_t, qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS, qemu_plugin_id_t,
        qemu_plugin_insn, qemu_plugin_mem_rw_QEMU_PLUGIN_MEM_R, qemu_plugin_meminfo_t,
        qemu_plugin_register_atexit_cb, qemu_plugin_register_flush_cb,
        qemu_plugin_register_vcpu_exit_cb, qemu_plugin_register_vcpu_idle_cb,
        qemu_plugin_register_vcpu_init_cb, qemu_plugin_register_vcpu_insn_exec_cb,
        qemu_plugin_register_vcpu_mem_cb, qemu_plugin_register_vcpu_resume_cb,
        qemu_plugin_register_vcpu_syscall_cb, qemu_plugin_register_vcpu_syscall_ret_cb,
        qemu_plugin_register_vcpu_tb_exec_cb, qemu_plugin_register_vcpu_tb_trans_cb,
        qemu_plugin_tb,
    },
    args::Args,
};

/// Trait for a callback that registers itself with QEMU during plugin installation
pub trait Register {
    /// Register the callback with QEMU for the given plugin ID
    ///
    /// # Arguments
    ///
    /// * `id` - The plugin ID to register the callback with
    fn register(&self, id: u64);
}

/// Trait for a callback registered dynamically and associated with a particular translation block
pub trait RegisterTBExec {
    /// Register the callback with QEMU for the given translation block
    ///
    /// # Arguments
    ///
    /// * `tb` - The translation block to register the callback with
    fn register(&self, tb: *mut qemu_plugin_tb);
}

/// Trait for a callback registered dynamically and associated with a particular instruction
pub trait RegisterInsnExec {
    /// Register the callback with QEMU for the given instruction
    ///
    /// # Arguments
    ///
    /// * `insn` - The instruction to register the callback with
    fn register(&self, insn: *mut qemu_plugin_insn);
}

/// First callback fired on installation of the plugin and allows configuration of global state
/// for the plugin
pub struct SetupCallback {
    /// Callback receiving a pointer the qemu info struct and the arguments passed to the plugin
    pub cb: Box<dyn Fn(*const qemu_info_t, &Args) + Send + Sync>,
}

impl SetupCallback {
    /// Instantiate a new `SetupCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving a pointer the qemu info struct and the arguments passed to the plugin
    pub fn new(cb: impl Fn(*const qemu_info_t, &Args) + Send + Sync + 'static) -> Self {
        Self { cb: Box::new(cb) }
    }
}

/// Enum wrapper for the setup callback and other non-QEMU callbacks
pub enum SetupCallbackType {
    /// A setup callback
    Setup(&'static Lazy<SetupCallback>),
}

/// Callback fired when a VCPU is initialized
pub struct VCPUInitCallback {
    /// Callback receiving the plugin id and the vcpu id
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

/// Callback fired when a VCPU is initialized. In user mode, this only happens once, but in
/// system mode this can happen any number of times
impl VCPUInitCallback {
    /// Instantiate a new `VCPUInitCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id and the vcpu id
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUInitCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_init_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when a VCPU exits. In user mode, this only happens once, but in
/// system mode this can happen any number of times
pub struct VCPUExitCallback {
    /// Callback receiving the plugin id and the vcpu id
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUExitCallback {
    /// Instantiate a new `VCPUExitCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id and the vcpu id
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUExitCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_exit_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when a VCPU starts to idle. This is only fired in system mode
pub struct VCPUIdleCallback {
    /// Callback receiving the plugin id and the vcpu id
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUIdleCallback {
    /// Instantiate a new `VCPUIdleCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id and the vcpu id
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

/// Callback fired when a VCPU resumes from idle. This is only fired in system mode
impl Register for VCPUIdleCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_idle_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when a VCPU resumes from idle. This is only fired in system mode
pub struct VCPUResumeCallback {
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUResumeCallback {
    /// Instantiate a new `VCPUResumeCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id and the vcpu id
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUResumeCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_resume_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when translation block is translated by TCG
pub struct VCPUTBTransCallback {
    /// Callback receiving the plugin id and a pointer to the *opaque* translation block object
    pub cb: unsafe extern "C" fn(u64, *mut qemu_plugin_tb) -> (),
}

impl VCPUTBTransCallback {
    /// Instantiate a new `VCPUTBTransCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id and a pointer to the *opaque* translation block object
    pub fn new(cb: unsafe extern "C" fn(u64, *mut qemu_plugin_tb) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUTBTransCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_tb_trans_cb(id, Some(self.cb)) };
    }
}

/// Callback fired when a system call is executed
pub struct VCPUSyscallCallback {
    /// Callback receiving the plugin id, vcpu id, syscall number, and arguments 0 through 7
    pub cb: unsafe extern "C" fn(u64, u32, i64, u64, u64, u64, u64, u64, u64, u64, u64) -> (),
}

impl VCPUSyscallCallback {
    /// Instantiate a new `VCPUSyscallCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id, vcpu id, syscall number, and arguments 0 through 7
    ///
    /// The system call's return `VCPUSyscallRetCallback` will be the next callback fired with
    /// the same plugin id and vcpu id, and the return value of the system call can be associated
    /// with its arguments by tracking the next return callback with the same plugin id and vcpu id
    /// as this system call callback.
    pub fn new(
        cb: unsafe extern "C" fn(u64, u32, i64, u64, u64, u64, u64, u64, u64, u64, u64) -> (),
    ) -> Self {
        Self { cb }
    }
}

impl Register for VCPUSyscallCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_syscall_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when a system call returns
pub struct VCPUSyscallRetCallback {
    /// Callback receiving the plugin id, vcpu id, system call number, and the return value
    /// of the system call
    pub cb: unsafe extern "C" fn(u64, u32, i64, i64) -> (),
}

impl VCPUSyscallRetCallback {
    /// Instantiate a new `VCPUSyscallRetCallback` with the given callback
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id, vcpu id, syscall number, and return value
    ///
    /// This callback will be the the next callback fired after the `VCPUSyscallCallback` callback
    /// for the same vcpu id and plugin id. Therefore it is sufficient to track these two values
    /// to determine which syscall is returning and associate a return value to the arguments.
    pub fn new(cb: unsafe extern "C" fn(u64, u32, i64, i64) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUSyscallRetCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_syscall_ret_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when the plugin exits. Unless manually unregistered, this callback will be fired
/// when QEMU exits.
pub struct AtExitCallback<T>
where
    T: Send + Sync + Into<*mut c_void> + 'static,
{
    /// Callback receiving the plugin id and a pointer to `data`
    pub cb: unsafe extern "C" fn(u64, *mut c_void) -> (),
    /// The data passed to `cb` when it is fired
    pub data: T,
}

impl<T> AtExitCallback<T>
where
    T: Send + Sync + Into<*mut c_void> + 'static,
{
    /// Instantiate a new `AtExitCallback` with the given callback and data
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the plugin id and a pointer to `data`
    /// * `data` - The data passed to `cb` when it is fired, this can be anything and will
    ///           be passed to `cb` as a pointer to the original `data` value
    pub fn new(cb: unsafe extern "C" fn(u64, *mut c_void) -> (), data: T) -> Self {
        Self { cb, data }
    }
}

impl<T> Register for AtExitCallback<T>
where
    T: Send + Sync + Into<*mut c_void> + 'static,
{
    fn register(&self, id: u64) {
        unsafe {
            qemu_plugin_register_atexit_cb(
                id as qemu_plugin_id_t,
                Some(self.cb),
                Box::into_raw(Box::new(&self.data)) as *mut c_void,
            )
        };
    }
}

pub struct AtExitData(*mut c_void);

unsafe impl Send for AtExitData {}
unsafe impl Sync for AtExitData {}

impl Into<*mut c_void> for AtExitData {
    fn into(self) -> *mut c_void {
        self.0
    }
}

// TODO: Document flush callback
/// Callback fired when ??? (No documentation in QEMU on when exactly a flush occurs). Please
/// open an issue if you know what this callback is for!
pub struct FlushCallback {
    pub cb: unsafe extern "C" fn(u64) -> (),
}

impl FlushCallback {
    pub fn new(cb: unsafe extern "C" fn(u64) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for FlushCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_flush_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Variant container for static callbacks that are called when a plugin is loaded
pub enum StaticCallbackType {
    VCPUInit(&'static Lazy<VCPUInitCallback>),
    VCPUExit(&'static Lazy<VCPUExitCallback>),
    VCPUIdle(&'static Lazy<VCPUIdleCallback>),
    VCPUResume(&'static Lazy<VCPUResumeCallback>),
    VCPUTBTrans(&'static Lazy<VCPUTBTransCallback>),
    VCPUSyscall(&'static Lazy<VCPUSyscallCallback>),
    VCPUSyscallRet(&'static Lazy<VCPUSyscallRetCallback>),
    AtExit(&'static Lazy<AtExitCallback<AtExitData>>),
    Flush(&'static Lazy<FlushCallback>),
}

impl Register for StaticCallbackType {
    fn register(&self, id: u64) {
        match self {
            StaticCallbackType::VCPUInit(cb) => cb.register(id),
            StaticCallbackType::VCPUExit(cb) => cb.register(id),
            StaticCallbackType::VCPUIdle(cb) => cb.register(id),
            StaticCallbackType::VCPUResume(cb) => cb.register(id),
            StaticCallbackType::VCPUTBTrans(cb) => cb.register(id),
            StaticCallbackType::VCPUSyscall(cb) => cb.register(id),
            StaticCallbackType::VCPUSyscallRet(cb) => cb.register(id),
            StaticCallbackType::AtExit(cb) => cb.register(id),
            StaticCallbackType::Flush(cb) => cb.register(id),
        }
    }
}

/// Callback fired when a translation block is executed
pub struct VCPUTBExecCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    /// Callback receiving the vcpu id and a pointer to the `data` field
    pub cb: unsafe extern "C" fn(u32, *mut c_void) -> (),
    /// Data passed to `cb` when it is fired
    pub data: T,
}

impl<T> VCPUTBExecCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    /// Instantiate a new `VCPUTBExecCallback` with the given callback and data
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the vcpu id and a pointer to the `data` field
    /// * `data` - Data passed to `cb` when it is fired, this can be anything and will
    ///           be passed to `cb` as a pointer to the original `data` value
    pub fn new(cb: unsafe extern "C" fn(u32, *mut c_void) -> (), data: T) -> Self {
        Self { cb, data }
    }
}

impl<T> RegisterTBExec for VCPUTBExecCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    fn register(&self, tb: *mut qemu_plugin_tb) {
        let data = self.data.clone().into();
        unsafe {
            qemu_plugin_register_vcpu_tb_exec_cb(
                tb,
                Some(self.cb),
                qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS,
                data,
            )
        };
    }
}

/// Callback fired when a translated instruction is executed
pub struct VCPUInsnExecCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    /// Callback receiving the vcpu id and a pointer to the `data` field
    pub cb: unsafe extern "C" fn(u32, *mut c_void) -> (),
    /// Data passed to `cb` when it is fired
    pub data: T,
}

impl<T> VCPUInsnExecCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    /// Instantiate a new `VCPUInsnExecCallback` with the given callback and data
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the vcpu id and a pointer to the `data` field
    /// * `data` - Data passed to `cb` when it is fired, this can be anything and will
    ///           be passed to `cb` as a pointer to the original `data` value
    pub fn new(cb: unsafe extern "C" fn(u32, *mut c_void) -> (), data: T) -> Self {
        Self { cb, data }
    }
}

impl<T> RegisterInsnExec for VCPUInsnExecCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    fn register(&self, insn: *mut qemu_plugin_insn) {
        let data: *mut c_void = self.data.clone().into();
        unsafe {
            qemu_plugin_register_vcpu_insn_exec_cb(
                insn,
                Some(self.cb),
                qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS,
                data,
            );
        };
    }
}

/// callback fired when a memory access is made by a translated instruction
pub struct VCPUMemCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    /// Callback receiving the vcpu id, the opaque memory info object, the virtual address of the
    /// memory access, and a pointer to the `data` field
    pub cb: unsafe extern "C" fn(u32, qemu_plugin_meminfo_t, u64, *mut c_void) -> (),
    /// Data passed to `cb` when it is fired
    pub data: T,
}

impl<T> VCPUMemCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    /// Instantiate a new `VCPUMemCallback` with the given callback and data
    ///
    /// # Arguments
    ///
    /// * `cb` - Callback receiving the vcpu id, the opaque memory info object, the virtual address of the
    ///          memory access, and a pointer to the `data` field
    /// * `data` - Data passed to `cb` when it is fired, this can be anything and will
    ///           be passed to `cb` as a pointer to the original `data` value
    pub fn new(
        cb: unsafe extern "C" fn(u32, qemu_plugin_meminfo_t, u64, *mut c_void) -> (),
        data: T,
    ) -> Self {
        Self { cb, data }
    }
}

impl<T> RegisterInsnExec for VCPUMemCallback<T>
where
    T: Send + Sync + Clone + Into<*mut c_void> + 'static,
{
    fn register(&self, insn: *mut qemu_plugin_insn) {
        let data = self.data.clone().into();
        unsafe {
            qemu_plugin_register_vcpu_mem_cb(
                insn,
                Some(self.cb),
                qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS,
                qemu_plugin_mem_rw_QEMU_PLUGIN_MEM_R,
                data,
            );
        };
    }
}
