use bitflags::bitflags;
use bytes::{Buf, BufMut, BytesMut};
use rand::{thread_rng, Rng};
use serde::Serialize;
use std::mem::size_of;
use tokio_util::codec::{Decoder, Encoder};

// The maximum opcode size on x86_64 + 1, which is the maximum size of an
// opcode on any reasonable architecture. This may be increased later if we
// find out another arch uses a larger opcode.
pub const MAX_OPCODE_SIZE: usize = 16;
/// The number of syscall arguments QEMu exposes to the plugin.
pub const NUM_SYSCALL_ARGS: usize = 8;

/// Trait that defines serialization of a structure to go over the wire with a Frame Codec
pub trait ToBytes {
    fn to_bytes(&self, bytes: &mut BytesMut);
}

/// Trait that defines deserialization of a structure to come from the wire with a Frame Codec
pub trait FromBytes {
    fn from_bytes(bytes: &mut BytesMut) -> Self;
}

bitflags! {
    #[repr(C)]
    #[derive(Default, Serialize)]
    /// Flags that are used to indicate what event types are enabled or what event types an event
    /// actually contains
    pub struct EventFlags: u32 {
        // Flag to trace PC
        const PC           = 0b00000001;
        // Flag to trace Reads & Writes (no additional overhead, so zero reason not to combine)
        const READS_WRITES = 0b00000010;
        // Flag to trace Instructions
        const INSTRS       = 0b00001000;
        // Flag to trace Syscalls
        const SYSCALLS     = 0b00010000;
        // Flag to trace Branches
        const BRANCHES     = 0b00100000;
        // Flag that an event has executed (used internally by the QEMU plugin)
        const EXECUTED     = 0b01000000;
        // Flag that QEMU has finished executing
        const FINISHED     = 0b10000000;
        /// Flag that the event is a program load event
        const LOAD         = 0b0000000100000000;
    }
}

impl EventFlags {
    /// Construct an `EventFlags` object from boolean flags
    pub fn from(
        has_pc: bool,
        has_instrs: bool,
        has_reads_writes: bool,
        has_syscalls: bool,
        has_branches: bool,
    ) -> Self {
        let mut flags = EventFlags::default();
        if has_pc {
            flags |= EventFlags::PC;
        }
        if has_instrs {
            flags |= EventFlags::INSTRS;
        }
        if has_reads_writes {
            flags |= EventFlags::READS_WRITES;
        }
        if has_syscalls {
            flags |= EventFlags::SYSCALLS;
        }
        if has_branches {
            flags |= EventFlags::BRANCHES;
        }
        flags
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize)]
/// The Qemu program counter event
pub struct QemuPc {
    /// The program counter value. If the event has the PC flag, this value will be set to
    /// the program counter of the instruction
    pub pc: u64,
    /// Whether this instruction occurs at a branch
    pub branch: bool,
}

impl QemuPc {
    /// Construct a new `QemuPc` object
    pub fn new(pc: u64, branch: bool) -> Self {
        Self { pc, branch }
    }

    /// Create a new random `QemuPc` object for debugging and benchmarking purposes
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        Self {
            pc: rng.gen(),
            branch: rng.gen(),
        }
    }
}

impl ToBytes for QemuPc {
    /// Serialize the `QemuPc` object to bytes
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u64(self.pc);
        bytes.put_u8(self.branch as u8);
    }
}

impl FromBytes for QemuPc {
    /// Deserialize the `QemuPc` object from bytes
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        Self {
            pc: bytes.get_u64(),
            branch: bytes.get_u8() != 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
/// The Instruction event
pub struct QemuInstr {
    /// The full instruction opcode bytes
    pub opcode: [u8; MAX_OPCODE_SIZE],
    /// The size of the opcode in bytes - the `opcode` array may be larger
    /// than the actual opcode size, this is the actual size of the opcode
    pub opcode_size: usize,
    // NOTE: QEMU supports obtaining disassembly of the instruction, but
    // it uses Capstone, which is known to be very slow. To avoid bottlenecking
    // QEMU, we don't disassemble and instead defer doing so to a consumer
    // of the event stream.
}

impl QemuInstr {
    /// Construct a new `QemuInstr` object
    pub fn new(opcode: [u8; MAX_OPCODE_SIZE], opcode_size: usize) -> Self {
        Self {
            opcode,
            opcode_size,
        }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        let mut opcode = [0u8; MAX_OPCODE_SIZE];
        for i in 0..MAX_OPCODE_SIZE {
            opcode[i] = rng.gen();
        }

        Self {
            opcode,
            opcode_size: rng.gen_range(1..MAX_OPCODE_SIZE),
        }
    }
}

impl ToBytes for QemuInstr {
    /// Serialize the `QemuInstr` object to bytes
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_slice(&self.opcode[..]);
        bytes.put_u64(self.opcode_size as u64);
    }
}

impl FromBytes for QemuInstr {
    /// Deserialize the `QemuInstr` object from bytes
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let mut opcode = [0u8; MAX_OPCODE_SIZE];
        bytes.copy_to_slice(&mut opcode[..]);
        let opcode_size = bytes.get_u64() as usize;
        QemuInstr {
            opcode,
            opcode_size,
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
/// The read event
pub struct QemuMemAccess {
    /// PC of the instruction that caused the read/write
    pub pc: u64,
    /// The virtual address of the read/write
    pub addr: u64,
    /// Whether it was a read or write
    pub is_write: bool,
}

impl QemuMemAccess {
    /// Construct a new `QemuMemAccess` object
    pub fn new(pc: u64, addr: u64, is_write: bool) -> Self {
        Self { pc, addr, is_write }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        Self {
            pc: rng.gen(),
            addr: rng.gen_range(0..u64::MAX),
            is_write: rng.gen(),
        }
    }
}

impl ToBytes for QemuMemAccess {
    /// Serialize the `QemuMemAccess` object to bytes
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u64(self.pc);
        bytes.put_u64(self.addr);
        bytes.put_u8(self.is_write as u8);
    }
}

impl FromBytes for QemuMemAccess {
    /// Deserialize the `QemuMemAccess` object from bytes
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let pc = bytes.get_u64();
        let addr = bytes.get_u64();
        let is_write = bytes.get_u8() != 0;
        QemuMemAccess { pc, addr, is_write }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
/// The syscall event
/// We don't know PC information, but we ensure we lock the sender when we submit messages
pub struct QemuSyscall {
    /// The syscall number that was executed
    pub num: i64,
    /// The return value of the syscall
    pub rv: i64,
    /// The syscall arguments (NOTE: any pointers are not visible)
    pub args: [u64; NUM_SYSCALL_ARGS],
}

impl QemuSyscall {
    /// Construct a new `QemuSyscall` object
    pub fn new(num: i64, rv: i64, args: [u64; 8]) -> Self {
        Self { num, rv, args }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        let mut args = [0u64; 8];

        for i in 0..NUM_SYSCALL_ARGS {
            args[i] = rng.gen_range(0..u64::MAX);
        }

        Self {
            num: rng.gen_range(0..i64::MAX),
            rv: rng.gen_range(0..i64::MAX),
            args,
        }
    }
}

impl ToBytes for QemuSyscall {
    /// Serialize the `QemuSyscall` object to bytes
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_i64(self.num);
        bytes.put_i64(self.rv);

        for arg in self.args.iter() {
            bytes.put_u64(*arg);
        }
    }
}

impl FromBytes for QemuSyscall {
    /// Deserialize the `QemuSyscall` object from bytes
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let num = bytes.get_i64();
        let rv = bytes.get_i64();

        let mut args = [0u64; 8];

        for arg in args.iter_mut() {
            *arg = bytes.get_u64();
        }

        QemuSyscall { num, rv, args }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
// The load event
pub struct QemuLoad {
    pub min: u64,
    pub max: u64,
    /// Only set if this is the main object
    pub entry: u64,
    pub prot: u8,
}

impl QemuLoad {
    /// Construct a new `QemuLoad` object
    pub fn new(min: u64, max: u64, entry: u64, prot: u8) -> Self {
        Self {
            min,
            max,
            entry,
            prot,
        }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();

        Self {
            min: rng.gen_range(0..u64::MAX),
            max: rng.gen_range(0..u64::MAX),
            entry: rng.gen_range(0..u64::MAX),
            prot: rng.gen_range(0..u8::MAX),
        }
    }
}

impl ToBytes for QemuLoad {
    /// Serialize the `QemuLoad` object to bytes
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u64(self.min);
        bytes.put_u64(self.max);
        bytes.put_u64(self.entry);
        bytes.put_u8(self.prot);
    }
}

impl FromBytes for QemuLoad {
    /// Deserialize the `QemuLoad` object from bytes
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let min = bytes.get_u64();
        let max = bytes.get_u64();
        let entry = bytes.get_u64();
        let prot = bytes.get_u8();

        QemuLoad {
            min,
            max,
            entry,
            prot,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
pub enum QemuEvent {
    /// The program counter event
    Pc(QemuPc),
    /// The instruction event
    Instr(QemuInstr),
    /// The read event
    MemAccess(QemuMemAccess),
    /// The syscall event
    Syscall(QemuSyscall),
    /// The load event
    Load(QemuLoad),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
/// The event message
pub struct QemuEventMsg {
    /// The flags indicating which event is present
    pub flags: EventFlags,
    /// The event
    pub event: QemuEvent,
}

impl QemuEventMsg {
    /// Construct a new `QemuEventMsg` object
    pub fn new(flags: EventFlags, event: QemuEvent) -> Self {
        Self { flags, event }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        let flags = EventFlags::from_bits_truncate(rng.gen_range(0..u32::MAX));
        let event = match rng.gen_range(0..7) {
            0 => QemuEvent::Pc(QemuPc::new_random()),
            1 => QemuEvent::Instr(QemuInstr::new_random()),
            2 => QemuEvent::MemAccess(QemuMemAccess::new_random()),
            4 => QemuEvent::Syscall(QemuSyscall::new_random()),
            6 => QemuEvent::Load(QemuLoad::new_random()),
            _ => unreachable!(),
        };

        Self { flags, event }
    }
}

impl ToBytes for QemuEventMsg {
    /// Serialize the `QemuEventMsg` object to bytes
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u32(self.flags.bits());
        match self.event {
            QemuEvent::Pc(ref event) => event.to_bytes(bytes),
            QemuEvent::Instr(ref event) => event.to_bytes(bytes),
            QemuEvent::MemAccess(ref event) => event.to_bytes(bytes),
            QemuEvent::Syscall(ref event) => event.to_bytes(bytes),
            QemuEvent::Load(ref event) => event.to_bytes(bytes),
        }
    }
}

impl FromBytes for QemuEventMsg {
    /// Deserialize the `QemuEventMsg` object from bytes
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let flags = EventFlags::from_bits_truncate(bytes.get_u32());
        let event = if flags.contains(EventFlags::PC) {
            QemuEvent::Pc(QemuPc::from_bytes(bytes))
        } else if flags.contains(EventFlags::INSTRS) {
            QemuEvent::Instr(QemuInstr::from_bytes(bytes))
        } else if flags.contains(EventFlags::READS_WRITES) {
            QemuEvent::MemAccess(QemuMemAccess::from_bytes(bytes))
        } else if flags.contains(EventFlags::SYSCALLS) {
            QemuEvent::Syscall(QemuSyscall::from_bytes(bytes))
        } else if flags.contains(EventFlags::LOAD) {
            QemuEvent::Load(QemuLoad::from_bytes(bytes))
        } else {
            unreachable!()
        };

        QemuEventMsg { flags, event }
    }
}

/// Codec for serializing/deserializing the `QemuEventExec` object to/from bytes
pub struct QemuMsgCodec {}

impl Encoder<QemuEventMsg> for QemuMsgCodec {
    type Error = std::io::Error;

    /// Encode the `QemuEventExec` object to bytes
    fn encode(&mut self, item: QemuEventMsg, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.to_bytes(dst);
        Ok(())
    }
}

impl Decoder for QemuMsgCodec {
    type Item = QemuEventMsg;
    type Error = std::io::Error;

    /// Decode a `QemuEventExec` object from bytes
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < size_of::<QemuEventMsg>() {
            return Ok(None);
        }

        let exec = QemuEventMsg::from_bytes(src);
        return Ok(Some(exec));
    }
}
