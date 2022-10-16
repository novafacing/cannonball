use bitflags::bitflags;
use bytes::{Buf, BufMut, BytesMut};
use rand::{thread_rng, Rng};
use std::mem::size_of;
use tokio_util::codec::{Decoder, Encoder};

// The maximum opcode size on x86_64 + 1, which is the maximum size of an
// opcode on any reasonable architecture. This may be increased later if we
// find out another arch uses a larger opcode.
pub const MAX_OPCODE_SIZE: usize = 16;
pub const NUM_SYSCALL_ARGS: usize = 8;

pub trait ToBytes {
    fn to_bytes(&self, bytes: &mut BytesMut);
}

pub trait FromBytes {
    fn from_bytes(bytes: &mut BytesMut) -> Self;
}

bitflags! {
    #[repr(C)]
    #[derive(Default)]
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
    }
}

impl EventFlags {
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
#[derive(Debug, Clone, Copy)]
pub struct QemuPc {
    pub pc: u64,
}

impl QemuPc {
    pub fn new(pc: u64) -> Self {
        Self { pc }
    }

    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        Self { pc: rng.gen() }
    }
}

impl ToBytes for QemuPc {
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u64(self.pc);
    }
}

impl FromBytes for QemuPc {
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        Self {
            pc: bytes.get_u64(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QemuInstr {
    // The full instruction opcode bytes
    pub opcode: [u8; MAX_OPCODE_SIZE],
    // The size of the opcode in bytes - the `opcode` array may be larger
    // than the actual opcode size, this is the actual size of the opcode
    pub opcode_size: usize,
    // NOTE: QEMU supports obtaining disassembly of the instruction, but
    // it uses Capstone, which is known to be very slow. To avoid bottlenecking
    // QEMU, we don't disassemble and instead defer doing so to a consumer
    // of the event stream.
}

impl QemuInstr {
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
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_slice(&self.opcode[..]);
        bytes.put_u64(self.opcode_size as u64);
    }
}

impl FromBytes for QemuInstr {
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
#[derive(Copy, Clone, Debug)]
pub struct QemuRead {
    /// The virtual address of the read
    addr: u64,
}

impl QemuRead {
    pub fn new(addr: u64) -> Self {
        Self { addr }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        Self {
            addr: rng.gen_range(0..u64::MAX),
        }
    }
}

impl ToBytes for QemuRead {
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u64(self.addr);
    }
}

impl FromBytes for QemuRead {
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let addr = bytes.get_u64();
        QemuRead { addr }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QemuWrite {
    /// The virtual address of the write
    addr: u64,
}

impl QemuWrite {
    pub fn new(addr: u64) -> Self {
        Self { addr }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        Self {
            addr: rng.gen_range(0..u64::MAX),
        }
    }
}

impl ToBytes for QemuWrite {
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u64(self.addr);
    }
}

impl FromBytes for QemuWrite {
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let addr = bytes.get_u64();
        QemuWrite { addr }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QemuSyscall {
    /// The syscall number that was executed
    num: i64,
    /// The return value of the syscall
    rv: i64,
    /// The syscall arguments (NOTE: any pointers are not visible)
    args: [u64; NUM_SYSCALL_ARGS],
}

impl QemuSyscall {
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
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_i64(self.num);
        bytes.put_i64(self.rv);

        for arg in self.args.iter() {
            bytes.put_u64(*arg);
        }
    }
}

impl FromBytes for QemuSyscall {
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
#[derive(Copy, Clone, Debug)]
pub struct QemuBranch {
    branch: bool,
}

impl QemuBranch {
    pub fn new(branch: bool) -> Self {
        Self { branch }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        Self { branch: rng.gen() }
    }
}

impl ToBytes for QemuBranch {
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u8(self.branch as u8);
    }
}

impl FromBytes for QemuBranch {
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let branch = bytes.get_u8() != 0;
        QemuBranch { branch }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QemuEventExec {
    // This is a C struct, we can't just use Option<> easily so we just flag whether or not
    // the fields are valid -- everything is pretty small so this is fine...FOR NOW
    // TODO: Make this structure more efficient if possible
    pub flags: EventFlags,

    /// The program counter of the execution
    pc: QemuPc,
    instr: QemuInstr,
    read: QemuRead,
    write: QemuWrite,
    syscall: QemuSyscall,
    branch: QemuBranch,
}

impl QemuEventExec {
    pub fn new(
        pc: Option<QemuPc>,
        instr: Option<QemuInstr>,
        read: Option<QemuRead>,
        write: Option<QemuWrite>,
        syscall: Option<QemuSyscall>,
        branch: Option<QemuBranch>,
    ) -> Self {
        let (has_pc, pc) = match pc {
            Some(pc) => (true, pc),
            None => (false, QemuPc::new(0)),
        };
        let (has_instrs, instr) = match instr {
            Some(instr) => (true, instr),
            None => (false, QemuInstr::new([0u8; MAX_OPCODE_SIZE], 0)),
        };
        let (has_reads, read) = match read {
            Some(read) => (true, read),
            None => (false, QemuRead::new(0)),
        };
        let (has_writes, write) = match write {
            Some(write) => (true, write),
            None => (false, QemuWrite::new(0)),
        };
        let (has_syscalls, syscall) = match syscall {
            Some(syscall) => (true, syscall),
            None => (false, QemuSyscall::new(0, 0, [0; 8])),
        };
        let (has_branches, branch) = match branch {
            Some(branch) => (true, branch),
            None => (false, QemuBranch::new(false)),
        };

        let has_reads_writes = has_reads || has_writes;

        let flags: EventFlags = EventFlags::from(
            has_pc,
            has_instrs,
            has_reads_writes,
            has_syscalls,
            has_branches,
        );

        Self {
            flags,
            pc,
            instr,
            read,
            write,
            syscall,
            branch,
        }
    }

    /// For performance testing only
    pub fn new_random() -> Self {
        let mut rng = thread_rng();
        let pc = if rng.gen() {
            Some(QemuPc::new_random())
        } else {
            None
        };
        let instr = if rng.gen() {
            Some(QemuInstr::new_random())
        } else {
            None
        };
        let read = if rng.gen() {
            Some(QemuRead::new_random())
        } else {
            None
        };
        let write = if rng.gen() {
            Some(QemuWrite::new_random())
        } else {
            None
        };
        let syscall = if rng.gen() {
            Some(QemuSyscall::new_random())
        } else {
            None
        };
        let branch = if rng.gen() {
            Some(QemuBranch::new_random())
        } else {
            None
        };

        Self::new(pc, instr, read, write, syscall, branch)
    }
}

impl ToBytes for QemuEventExec {
    fn to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put_u32(self.flags.bits());
        self.pc.to_bytes(bytes);
        self.instr.to_bytes(bytes);
        self.read.to_bytes(bytes);
        self.write.to_bytes(bytes);
        self.syscall.to_bytes(bytes);
        self.branch.to_bytes(bytes);
    }
}

impl FromBytes for QemuEventExec {
    fn from_bytes(bytes: &mut BytesMut) -> Self {
        let flags = EventFlags::from_bits_truncate(bytes.get_u32());
        let pc = QemuPc::from_bytes(bytes);
        let instr = QemuInstr::from_bytes(bytes);
        let read = QemuRead::from_bytes(bytes);
        let write = QemuWrite::from_bytes(bytes);
        let syscall = QemuSyscall::from_bytes(bytes);
        let branch = QemuBranch::from_bytes(bytes);

        QemuEventExec {
            flags,
            pc,
            instr,
            read,
            write,
            syscall,
            branch,
        }
    }
}

pub struct QemuEventCodec {}

impl Encoder<QemuEventExec> for QemuEventCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: QemuEventExec, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.to_bytes(dst);
        Ok(())
    }
}

impl Decoder for QemuEventCodec {
    type Item = QemuEventExec;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < size_of::<QemuEventExec>() {
            return Ok(None);
        }

        let exec = QemuEventExec::from_bytes(src);
        return Ok(Some(exec));
    }
}
