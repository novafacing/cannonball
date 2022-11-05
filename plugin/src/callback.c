#include <glib.h>
#include <string.h>

#include "callback.h"
#include "cannonball.h"
#include "error.h"
#include "logging.h"
#include "qemu-plugin.h"

#define likely(x) __builtin_expect((x), 1)
#define unlikely(x) __builtin_expect((x), 0)

/// The number of events per batch to send to the consumer
#define BATCH_SIZE (64)

/// We store other events indexec by their pointer (directly)
static GHashTable *events_htable = NULL;
/// Lock for the events hash table
static GMutex events_htable_lock;

/// We store mem events separately, indexed by their pointer (directly)
static GHashTable *mem_events_htable = NULL;
/// Lock for the mem events hash table
static GMutex mem_events_htable_lock;

// We store syscalls indexed by vcpu, because a vcpu can only execute one
// syscall and return from it at a time (we hope!)
static GHashTable *syscall_htable = NULL;
/// Lock for the syscall event
static GMutex syscall_htable_lock;

/// The sender we use to send events to the consumer
static Sender *sender = NULL;

/// Flags for enabled instrumentation
static EventFlags flags = {0};

/// Program info
static uint64_t start_code = 0;
static uint64_t end_code = 0;
static uint64_t entry_code = 0;

typedef struct QemuEventMsgMemWrapper {
    QemuEventMsg *msg;
    bool mem;
    bool exec;
} QemuEventMsgMemWrapper;

/// Set flags from a set of boolean values
#define SETFLAGS(f, p, rw, i, s, b)                                                    \
    do {                                                                               \
        f.bits |= ((p) ? EventFlags_PC.bits : 0);                                      \
        f.bits |= ((rw) ? EventFlags_READS_WRITES.bits : 0);                           \
        f.bits |= ((i) ? EventFlags_INSTRS.bits : 0);                                  \
        f.bits |= ((s) ? EventFlags_SYSCALLS.bits : 0);                                \
        f.bits |= ((b) ? EventFlags_BRANCHES.bits : 0);                                \
    } while (0)

/// Accessors for the flags
#define PC(f) (f.bits & EventFlags_PC.bits)
#define READS_WRITES(f) (f.bits & EventFlags_READS_WRITES.bits)
#define INSTRS(f) (f.bits & EventFlags_INSTRS.bits)
#define SYSCALLS(f) (f.bits & EventFlags_SYSCALLS.bits)
#define BRANCHES(f) (f.bits & EventFlags_BRANCHES.bits)
#define EXECUTED(f) (f.bits & EventFlags_EXECUTED.bits)
#define FINISHED(f) (f.bits & EventFlags_FINISHED.bits)
#define LOAD(f) (f.bits & EventFlags_LOAD.bits)

/// Setters for the flags
#define SETPC(f) (f.bits |= EventFlags_PC.bits)
#define SETREADS_WRITES(f) (f.bits |= EventFlags_READS_WRITES.bits)
#define SETINSTRS(f) (f.bits |= EventFlags_INSTRS.bits)
#define SETSYSCALLS(f) (f.bits |= EventFlags_SYSCALLS.bits)
#define SETBRANCHES(f) (f.bits |= EventFlags_BRANCHES.bits)
#define SETEXECUTED(f) (f.bits |= EventFlags_EXECUTED.bits)
#define SETFINISHED(f) (f.bits |= EventFlags_FINISHED.bits)
#define SETLOAD(f) (f.bits |= EventFlags_LOAD.bits)

/// An event is ready for submission if all requested instrumentation has been set on it
/// and it isn't a syscall event (because if it is it'll be ready on syscall ret and we
/// don't need to check for that)
#define READY(f, g)                                                                    \
    ((f.bits & ~EventFlags_SYSCALLS.bits) == (g.bits & ~EventFlags_SYSCALLS.bits))

/// Whether the instrumentation is set to track branches only
#define BRANCHONLY(f) (BRANCHES(f) && !PC(f) && !READS_WRITES(f) && !INSTRS(f))
/// Whether the instrumentation is set to not track instructions at all
#define NOINSN(f) (!PC(f) && !READS_WRITES(f) && !INSTRS(f) && !BRANCHES(f))

static QemuEventMsg *newpc(uint64_t pc, bool branch) {
    QemuEventMsg *evt = (QemuEventMsg *)calloc(1, sizeof(QemuEventMsg));
    SETPC(evt->flags);
    evt->event.tag = Pc;
    evt->event.pc.pc = pc;
    evt->event.pc.branch = branch;
    g_mutex_lock(&events_htable_lock);
    g_hash_table_insert(events_htable, evt, evt);
    g_mutex_unlock(&events_htable_lock);
    return evt;
}

static QemuEventMsg *newinstr(uint64_t pc, const void *data, uintptr_t opcode_size) {
    QemuEventMsg *evt = (QemuEventMsg *)calloc(1, sizeof(QemuEventMsg));
    SETINSTRS(evt->flags);
    evt->event.tag = Instr;
    evt->event.instr.pc = pc;
    evt->event.instr.opcode_size = opcode_size;
    memcpy(evt->event.instr.opcode, data, opcode_size);
    g_mutex_lock(&events_htable_lock);
    g_hash_table_insert(events_htable, evt, evt);
    g_mutex_unlock(&events_htable_lock);
    return evt;
}

// Mem accesses are tracked until the access happens by inserting into the hashset
static QemuEventMsgMemWrapper *newmemaccess(uint64_t pc, uint64_t addr, bool is_write) {
    QemuEventMsg *evt = (QemuEventMsg *)calloc(1, sizeof(QemuEventMsg));
    SETREADS_WRITES(evt->flags);
    evt->event.tag = MemAccess;
    evt->event.mem_access.pc = pc;
    evt->event.mem_access.addr = addr;
    evt->event.mem_access.is_write = is_write;
    QemuEventMsgMemWrapper *wrapper =
        (QemuEventMsgMemWrapper *)calloc(1, sizeof(QemuEventMsgMemWrapper));
    wrapper->msg = evt;
    wrapper->mem = false;
    wrapper->exec = false;
    g_mutex_lock(&mem_events_htable_lock);
    g_hash_table_insert(mem_events_htable, wrapper, wrapper);
    g_mutex_unlock(&mem_events_htable_lock);
    return wrapper;
}

static QemuEventMsg *newsyscall(unsigned int vcpu_index, uint64_t num, uint64_t arg0,
                                uint64_t arg1, uint64_t arg2, uint64_t arg3,
                                uint64_t arg4, uint64_t arg5, uint64_t arg6,
                                uint64_t arg7) {
    QemuEventMsg *evt = (QemuEventMsg *)calloc(1, sizeof(QemuEventMsg));
    SETSYSCALLS(evt->flags);
    evt->event.tag = Syscall;
    evt->event.syscall.num = num;
    // Placeholder, will be set before it is submitted
    evt->event.syscall.rv = -1;

    evt->event.syscall.args[0] = arg0;
    evt->event.syscall.args[1] = arg1;
    evt->event.syscall.args[2] = arg2;
    evt->event.syscall.args[3] = arg3;
    evt->event.syscall.args[4] = arg4;
    evt->event.syscall.args[5] = arg5;
    evt->event.syscall.args[6] = arg6;
    evt->event.syscall.args[7] = arg7;

    g_mutex_lock(&syscall_htable_lock);
    g_hash_table_replace(syscall_htable, GUINT_TO_POINTER(vcpu_index), evt);
    // Boot out an entry if one exists
    g_mutex_unlock(&syscall_htable_lock);

    return evt;
}

static QemuEventMsg *newload(uint64_t min, uint64_t max, uint64_t entry, uint8_t prot) {
    QemuEventMsg *evt = (QemuEventMsg *)calloc(1, sizeof(QemuEventMsg));
    SETLOAD(evt->flags);
    evt->event.tag = Load;
    evt->event.load.min = min;
    evt->event.load.max = max;
    evt->event.load.entry = entry;
    evt->event.load.prot = prot;
    return evt;
}

/// Callback executed when an instruction is actually executed
static void callback_on_insn_exec(unsigned int vcpu_index, void *userdata) {
    QemuEventMsg *msg = (QemuEventMsg *)userdata;
    g_mutex_lock(&events_htable_lock);
    if ((msg = g_hash_table_lookup(events_htable, msg))) {
        submit(sender, msg);
        g_hash_table_remove(events_htable, msg);
    }
    g_mutex_unlock(&events_htable_lock);
}

static void callback_on_insn_exec_mem(unsigned int vcpu_index, void *userdata) {
    QemuEventMsgMemWrapper *msg = (QemuEventMsgMemWrapper *)userdata;
    g_mutex_lock(&mem_events_htable_lock);
    if ((msg = g_hash_table_lookup(mem_events_htable, msg))) {
        msg->exec = true;
        if (msg->mem && msg->exec) {
            submit(sender, msg->msg);
            free(msg->msg);
            g_hash_table_remove(mem_events_htable, msg);
        }
    }
    g_mutex_unlock(&mem_events_htable_lock);
}

/// Callback executed when an instruction undergoes a memory access
static void callback_on_mem_access(unsigned int vcpu_index, qemu_plugin_meminfo_t info,
                                   uint64_t vaddr, void *userdata) {
    QemuEventMsgMemWrapper *msg = (QemuEventMsgMemWrapper *)userdata;
    g_mutex_lock(&mem_events_htable_lock);
    if ((msg = g_hash_table_lookup(mem_events_htable, msg))) {
        msg->mem = true;
        msg->msg->event.mem_access.addr = vaddr;
        msg->msg->event.mem_access.is_write = qemu_plugin_mem_is_store(info);

        if (msg->mem && msg->exec) {
            submit(sender, msg->msg);
            free(msg->msg);
            g_hash_table_remove(mem_events_htable, msg);
        }
    }
    g_mutex_unlock(&mem_events_htable_lock);
}

/// Callback executed when a translation block is translated to TCG instructions
static void callback_on_tb_trans(qemu_plugin_id_t id, struct qemu_plugin_tb *tb) {
    struct qemu_plugin_insn *insn = NULL;
    size_t num_insns = qemu_plugin_tb_n_insns(tb);

    if (unlikely(start_code == 0)) {
        start_code = qemu_plugin_start_code();
        end_code = qemu_plugin_end_code();
        entry_code = qemu_plugin_entry_code();
        QemuEventMsg *load_msg = newload(start_code, end_code, entry_code, 0x7);
        submit(sender, load_msg);
        free(load_msg);
    }

    for (size_t i = BRANCHONLY(flags) ? num_insns - 1 : 0; i < num_insns; i++) {

        insn = qemu_plugin_tb_get_insn(tb, i);
        uint64_t pc = qemu_plugin_insn_vaddr(insn);

        if (PC(flags)) {
            QemuEventMsg *pc_msg = newpc(pc, i == num_insns - 1);
            qemu_plugin_register_vcpu_insn_exec_cb(
                insn, callback_on_insn_exec, QEMU_PLUGIN_CB_NO_REGS, (void *)pc_msg);
        }

        if (INSTRS(flags)) {
            QemuEventMsg *instr_msg =
                newinstr(pc, qemu_plugin_insn_data(insn), qemu_plugin_insn_size(insn));
            qemu_plugin_register_vcpu_insn_exec_cb(
                insn, callback_on_insn_exec, QEMU_PLUGIN_CB_NO_REGS, (void *)instr_msg);
        }

        if (READS_WRITES(flags)) {
            QemuEventMsgMemWrapper *instr_msg = newmemaccess(pc, 0, false);
            qemu_plugin_register_vcpu_mem_cb(insn, callback_on_mem_access,
                                             QEMU_PLUGIN_CB_NO_REGS, QEMU_PLUGIN_MEM_R,
                                             (void *)instr_msg);
            qemu_plugin_register_vcpu_insn_exec_cb(insn, callback_on_insn_exec_mem,
                                                   QEMU_PLUGIN_CB_NO_REGS,
                                                   (void *)instr_msg);
        }
    }
}

/// Callback executed when a syscall is executed
static void callback_on_syscall(qemu_plugin_id_t id, unsigned int vcpu_index,
                                int64_t num, uint64_t a1, uint64_t a2, uint64_t a3,
                                uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
                                uint64_t a8) {
    newsyscall(vcpu_index, num, a1, a2, a3, a4, a5, a6, a7, a8);
}

/// Callback executed after a syscall returns
static void callback_after_syscall(qemu_plugin_id_t id, unsigned int vcpu_idx,
                                   int64_t num, int64_t ret) {

    /* If we are called, syscall tracing is active */
    g_mutex_lock(&syscall_htable_lock);
    QemuEventMsg *evt =
        (QemuEventMsg *)g_hash_table_lookup(syscall_htable, GUINT_TO_POINTER(vcpu_idx));
    if (evt && evt->event.syscall.num == num) {
        evt->event.syscall.rv = ret;
        submit(sender, evt);
    }
    g_hash_table_remove(syscall_htable, GUINT_TO_POINTER(vcpu_idx));
    g_mutex_unlock(&syscall_htable_lock);
}

static void callback_atexit(long unsigned int vcpu_idx, void *_) {
    log_info("VCPU %d exited, sending exit event.\n", vcpu_idx);

    teardown(sender);
}

/// Initialize the plugin's callbacks and set up the pipe to the consumer
ErrorCode callback_init(qemu_plugin_id_t id, bool trace_pc, bool trace_read,
                        bool trace_write, bool trace_instr, bool trace_syscall,
                        bool trace_branch, const char *socket_path) {

    SETFLAGS(flags, trace_pc, trace_read | trace_write, trace_instr, trace_syscall,
             trace_branch);

    g_mutex_lock(&events_htable_lock);
    if ((events_htable = g_hash_table_new_full(NULL, NULL, free, NULL)) == NULL) {
        log_error("Failed to allocate memory for events table.\n");
        g_mutex_unlock(&events_htable_lock);
        return OutOfMemory;
    }
    g_mutex_unlock(&events_htable_lock);

    g_mutex_lock(&mem_events_htable_lock);
    if ((mem_events_htable = g_hash_table_new_full(NULL, NULL, free, NULL)) == NULL) {
        log_error("Failed to allocate memory for mem events table.\n");
        g_mutex_unlock(&mem_events_htable_lock);
        return OutOfMemory;
    }
    g_mutex_unlock(&mem_events_htable_lock);

    g_mutex_lock(&syscall_htable_lock);
    if ((syscall_htable = g_hash_table_new_full(NULL, NULL, NULL, free)) == NULL) {
        log_error("Failed to allocate memory for syscall events table.\n");
        g_mutex_unlock(&syscall_htable_lock);
        return OutOfMemory;
    }
    g_mutex_unlock(&syscall_htable_lock);

    if ((sender = setup(BATCH_SIZE, socket_path)) == NULL) {
        log_error("Failed to setup sender.\n");

        return SenderInitError;
    }

    log_info("Initialized send pipe.\n");

    if (!NOINSN(flags)) {
        log_info("Registering callback for instruction execution\n");
        qemu_plugin_register_vcpu_tb_trans_cb(id, callback_on_tb_trans);
    }

    if (SYSCALLS(flags)) {
        qemu_plugin_register_vcpu_syscall_cb(id, callback_on_syscall);
        qemu_plugin_register_vcpu_syscall_ret_cb(id, callback_after_syscall);
        log_info("Registered syscall callbacks.\n");
    }

    log_info("Registering callback for vcpu exit\n");
    qemu_plugin_register_atexit_cb(id, callback_atexit, NULL);

    log_info("Initialized plugin callbacks.\n");

    return Success;
}
