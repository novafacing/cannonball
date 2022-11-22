use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InsnEvent {
    pub vcpu_idx: Option<u32>,
    pub vaddr: u64,
    pub opcode: Option<Vec<u8>>,
    pub branch: bool,
}

impl InsnEvent {
    /// Instantiate a new `InsnEvent` from the raw arguments passed to the plugin
    ///
    /// # Arguments
    ///
    /// * `vaddr` - The virtual address of the instruction
    /// * `opcode` - The opcode of the instruction, optional
    /// * `branch` - Whether or not the instruction is a branch (in this case, `branch`
    ///             is a bit of a misnomer -- it actually just means "last insn in the basic
    ///             block" not exclusively *conditional* branches)
    pub fn new(vcpu_idx: Option<u32>, vaddr: u64, opcode: Option<Vec<u8>>, branch: bool) -> Self {
        Self {
            vcpu_idx,
            vaddr,
            opcode,
            branch,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemEvent {
    pub vaddr: u64,
    pub is_sext: bool,
    pub is_be: bool,
    pub is_store: bool,
    pub size_shift: u32,
    pub insn: InsnEvent,
}

impl MemEvent {
    /// Instantiate a new `MemEvent` from the raw arguments passed to the plugin
    ///
    /// # Arguments
    ///
    /// * `vaddr` - The virtual address of the memory access
    /// * `is_sext` - Whether or not the memory access is sign extended
    /// * `is_be` - Whether or not the memory access is big endian
    /// * `is_store` - Whether or not the memory access is a store
    /// * `size_shift` - The size of the memory access, as a power of 2
    /// * `insn` - The instruction that caused the memory access
    pub fn new(
        vaddr: u64,
        is_sext: bool,
        is_be: bool,
        is_store: bool,
        size_shift: u32,
        insn: InsnEvent,
    ) -> Self {
        Self {
            vaddr,
            is_sext,
            is_be,
            is_store,
            size_shift,
            insn,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyscallEvent {
    pub num: i64,
    pub rv: Option<i64>,
    pub args: Vec<u64>,
}

impl SyscallEvent {
    pub fn new(num: i64, rv: Option<i64>, args: Vec<u64>) -> Self {
        Self { num, rv, args }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    Insn(InsnEvent),
    Mem(MemEvent),
    Syscall(SyscallEvent),
}
