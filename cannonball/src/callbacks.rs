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
//! use cannonball::callbacks::SetupCallback;
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

use std::{
    any::Any,
    sync::{Arc, Mutex},
};

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

/// Trait for a callback that registers itself with QEMU during plugin installation.
pub trait Register {
    fn register(&self, id: u64);
}

/// Trait for a callback registered dynamically and associated with a particular translation block.
pub trait RegisterTBExec {
    fn register(&self, tb: *mut qemu_plugin_tb);
}

/// Trait for a callback registered dynamically and associated with a particular instruction.
pub trait RegisterInsnExec {
    fn register(&self, insn: *mut qemu_plugin_insn);
}

/// First callback fired on installation of the plugin and allows configuration of global state
/// for the plugin.
pub struct SetupCallback {
    pub cb: Box<dyn Fn(*const qemu_info_t, &Args) + Send + Sync>,
}

impl SetupCallback {
    pub fn new(cb: impl Fn(*const qemu_info_t, &Args) + Send + Sync + 'static) -> Self {
        Self { cb: Box::new(cb) }
    }
}

pub enum SetupCallbackType {
    Setup(&'static Lazy<SetupCallback>),
}

/// Callback fired when a VCPU is initialized
pub struct VCPUInitCallback {
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUInitCallback {
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUInitCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_init_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

/// Callback fired when a VCPU exits
pub struct VCPUExitCallback {
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUExitCallback {
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUExitCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_exit_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

pub struct VCPUIdleCallback {
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUIdleCallback {
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUIdleCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_idle_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

pub struct VCPUResumeCallback {
    pub cb: unsafe extern "C" fn(u64, u32) -> (),
}

impl VCPUResumeCallback {
    pub fn new(cb: unsafe extern "C" fn(u64, u32) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUResumeCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_resume_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

pub struct VCPUTBTransCallback {
    pub cb: unsafe extern "C" fn(u64, *mut qemu_plugin_tb) -> (),
}

impl VCPUTBTransCallback {
    pub fn new(cb: unsafe extern "C" fn(u64, *mut qemu_plugin_tb) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUTBTransCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_tb_trans_cb(id, Some(self.cb)) };
    }
}

pub struct VCPUSyscallCallback {
    pub cb: unsafe extern "C" fn(u64, u32, i64, u64, u64, u64, u64, u64, u64, u64, u64) -> (),
}

impl VCPUSyscallCallback {
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

pub struct VCPUSyscallRetCallback {
    pub cb: unsafe extern "C" fn(u64, u32, i64, i64) -> (),
}

impl VCPUSyscallRetCallback {
    pub fn new(cb: unsafe extern "C" fn(u64, u32, i64, i64) -> ()) -> Self {
        Self { cb }
    }
}

impl Register for VCPUSyscallRetCallback {
    fn register(&self, id: u64) {
        unsafe { qemu_plugin_register_vcpu_syscall_ret_cb(id as qemu_plugin_id_t, Some(self.cb)) };
    }
}

pub struct AtExitCallback<T>
where
    T: Send + Sync,
{
    pub cb: unsafe extern "C" fn(u64, *mut c_void) -> (),
    pub data: Arc<Mutex<T>>,
}

impl<T> AtExitCallback<T>
where
    T: Send + Sync,
{
    pub fn new(cb: unsafe extern "C" fn(u64, *mut c_void) -> (), data: T) -> Self {
        Self {
            cb,
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl<T> Register for AtExitCallback<T>
where
    T: Send + Sync,
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

pub enum StaticCallbackType {
    VCPUInit(&'static Lazy<VCPUInitCallback>),
    VCPUExit(&'static Lazy<VCPUExitCallback>),
    VCPUIdle(&'static Lazy<VCPUIdleCallback>),
    VCPUResume(&'static Lazy<VCPUResumeCallback>),
    VCPUTBTrans(&'static Lazy<VCPUTBTransCallback>),
    VCPUSyscall(&'static Lazy<VCPUSyscallCallback>),
    VCPUSyscallRet(&'static Lazy<VCPUSyscallRetCallback>),
    AtExit(&'static Lazy<AtExitCallback<Box<dyn Any + Send + Sync>>>),
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

pub struct VCPUTBExecCallback<T>
where
    T: Any + Send + Sync,
{
    pub cb: unsafe extern "C" fn(u32, *mut c_void) -> (),
    pub data: Arc<Mutex<T>>,
}

impl<T> VCPUTBExecCallback<T>
where
    T: Any + Send + Sync,
{
    pub fn new(cb: unsafe extern "C" fn(u32, *mut c_void) -> (), data: T) -> Self {
        Self {
            cb,
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl<T> RegisterTBExec for VCPUTBExecCallback<T>
where
    T: Any + Send + Sync,
{
    fn register(&self, tb: *mut qemu_plugin_tb) {
        unsafe {
            qemu_plugin_register_vcpu_tb_exec_cb(
                tb,
                Some(self.cb),
                qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS,
                Box::into_raw(Box::new(&self.data)) as *mut c_void,
            )
        };
    }
}

pub struct VCPUInstrExecCallback<T>
where
    T: Any + Send + Sync,
{
    pub cb: unsafe extern "C" fn(u32, *mut c_void) -> (),
    pub data: Arc<Mutex<T>>,
}

impl<T> VCPUInstrExecCallback<T>
where
    T: Any + Send + Sync,
{
    pub fn new(cb: unsafe extern "C" fn(u32, *mut c_void) -> (), data: T) -> Self {
        Self {
            cb,
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl<T> RegisterInsnExec for VCPUInstrExecCallback<T>
where
    T: Any + Send + Sync,
{
    fn register(&self, insn: *mut qemu_plugin_insn) {
        unsafe {
            qemu_plugin_register_vcpu_insn_exec_cb(
                insn,
                Some(self.cb),
                qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS,
                Box::into_raw(Box::new(&self.data)) as *mut c_void,
            )
        };
    }
}

pub struct VCPUMemCallback<T>
where
    T: Any + Send + Sync,
{
    pub cb: unsafe extern "C" fn(u32, qemu_plugin_meminfo_t, u64, *mut c_void) -> (),
    pub data: Arc<Mutex<T>>,
}

impl<T> VCPUMemCallback<T>
where
    T: Any + Send + Sync,
{
    pub fn new(
        cb: unsafe extern "C" fn(u32, qemu_plugin_meminfo_t, u64, *mut c_void) -> (),
        data: T,
    ) -> Self {
        Self {
            cb,
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl<T> RegisterInsnExec for VCPUMemCallback<T>
where
    T: Any + Send + Sync,
{
    fn register(&self, insn: *mut qemu_plugin_insn) {
        unsafe {
            qemu_plugin_register_vcpu_mem_cb(
                insn,
                Some(self.cb),
                qemu_plugin_cb_flags_QEMU_PLUGIN_CB_NO_REGS,
                qemu_plugin_mem_rw_QEMU_PLUGIN_MEM_R,
                Box::into_raw(Box::new(&self.data)) as *mut c_void,
            );
        };
    }
}
