#include <glib.h>
#include <string.h>

#include "callback.h"
#include "cannonball-client.h"
#include "error.h"
#include "logging.h"

#define likely(x) __builtin_expect((x), 1)
#define unlikely(x) __builtin_expect((x), 0)

// The number of events per batch to send to the consumer
#define BATCH_SIZE (64)

// Temporaries for tracking events across multiple callbacks
static QemuEventExec *syscall_evt = NULL;
static GMutex syscall_evt_lock;
static GHashTable *exec_htable = NULL;
static GMutex exec_htable_lock;

static Sender *sender = NULL;

static EventFlags flags = {0};
#define SETFLAGS(f, p, rw, i, s, b)                                                    \
    do {                                                                               \
        f.bits |= ((p) ? EventFlags_PC.bits : 0);                                      \
        f.bits |= ((rw) ? EventFlags_READS_WRITES.bits : 0);                           \
        f.bits |= ((i) ? EventFlags_INSTRS.bits : 0);                                  \
        f.bits |= ((s) ? EventFlags_SYSCALLS.bits : 0);                                \
        f.bits |= ((b) ? EventFlags_BRANCHES.bits : 0);                                \
    } while (0)

#define PC(f) (f.bits & EventFlags_PC.bits)
#define READS_WRITES(f) (f.bits & EventFlags_READS_WRITES.bits)
#define INSTRS(f) (f.bits & EventFlags_INSTRS.bits)
#define SYSCALLS(f) (f.bits & EventFlags_SYSCALLS.bits)
#define BRANCHES(f) (f.bits & EventFlags_BRANCHES.bits)
#define EXECUTED(f) (f.bits & EventFlags_EXECUTED.bits)

#define SETPC(f) (f.bits |= EventFlags_PC.bits)
#define SETREADS_WRITES(f) (f.bits |= EventFlags_READS_WRITES.bits)
#define SETINSTRS(f) (f.bits |= EventFlags_INSTRS.bits)
#define SETSYSCALLS(f) (f.bits |= EventFlags_SYSCALLS.bits)
#define SETBRANCHES(f) (f.bits |= EventFlags_BRANCHES.bits)
#define SETEXECUTED(f) (f.bits |= EventFlags_EXECUTED.bits)

#define READY(f, g)                                                                    \
    ((f.bits & ~EventFlags_SYSCALLS.bits) == (g.bits & ~EventFlags_SYSCALLS.bits))

#define BRANCHONLY(f) (BRANCHES(f) && !PC(f) && !READS_WRITES(f) && !INSTRS(f))
#define NOINSN(f) (!PC(f) && !READS_WRITES(f) && !INSTRS(f) && !BRANCHES(f))

static void check_and_submit(QemuEventExec *event) {
    g_mutex_lock(&exec_htable_lock);
    if (READY(flags, event->flags) && g_hash_table_contains(exec_htable, event)) {
        g_hash_table_remove(exec_htable, event);
        submit(sender, event);
        free(event);
    }
    g_mutex_unlock(&exec_htable_lock);
}

static bool event_still_active(QemuEventExec *event) {
    bool rv = false;
    g_mutex_lock(&exec_htable_lock);
    if (g_hash_table_contains(exec_htable, event)) {
        rv = true;
    }
    g_mutex_unlock(&exec_htable_lock);
    return rv;
}

static void callback_on_insn_exec(unsigned int vcpu_index, void *userdata) {
    QemuEventExec *event = (QemuEventExec *)userdata;

    if (!event_still_active(event)) {
        return;
    }

    check_and_submit(event);
}

static void callback_on_mem_access(unsigned int vcpu_index, qemu_plugin_meminfo_t info,
                                   uint64_t vaddr, void *userdata) {
    QemuEventExec *event = (QemuEventExec *)userdata;

    if (!event_still_active(event)) {
        return;
    }

    if (qemu_plugin_mem_is_store(info)) {
        SETREADS_WRITES(event->flags);
        event->write.addr = vaddr;
    } else {
        SETREADS_WRITES(event->flags);
        event->read.addr = vaddr;
    }

    check_and_submit(event);
}

static void callback_on_tb_trans(qemu_plugin_id_t id, struct qemu_plugin_tb *tb) {
    struct qemu_plugin_insn *insn = NULL;
    size_t num_insns = qemu_plugin_tb_n_insns(tb);
    for (size_t i = BRANCHONLY(flags) ? num_insns - 1 : 0; i < num_insns; i++) {

        QemuEventExec *event = (QemuEventExec *)calloc(1, sizeof(QemuEventExec));
        g_mutex_lock(&exec_htable_lock);
        g_hash_table_insert(exec_htable, event, event);
        g_mutex_unlock(&exec_htable_lock);

        insn = qemu_plugin_tb_get_insn(tb, i);

        if (PC(flags)) {
            SETPC(event->flags);
            event->pc.pc = qemu_plugin_insn_vaddr(insn);
        }

        if (BRANCHES(flags) && i == num_insns - 1) {
            SETBRANCHES(event->flags);
            event->branch.branch = true;
            // Probably cheaper than conditionals?
            event->pc.pc = qemu_plugin_insn_vaddr(insn);
        }

        if (INSTRS(flags)) {
            SETINSTRS(event->flags);
            event->instr.opcode_size = qemu_plugin_insn_size(insn);
            memcpy(event->instr.opcode, qemu_plugin_insn_data(insn),
                   event->instr.opcode_size);
        }

        if (READS_WRITES(flags)) {
            qemu_plugin_register_vcpu_mem_cb(insn, callback_on_mem_access,
                                             QEMU_PLUGIN_CB_NO_REGS, QEMU_PLUGIN_MEM_R,
                                             (void *)event);
        }

        qemu_plugin_register_vcpu_insn_exec_cb(insn, callback_on_insn_exec,
                                               QEMU_PLUGIN_CB_NO_REGS, (void *)event);
    }
}

static void callback_on_syscall(qemu_plugin_id_t id, unsigned int vcpu_index,
                                int64_t num, uint64_t a1, uint64_t a2, uint64_t a3,
                                uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7,
                                uint64_t a8) {
    g_mutex_lock(&syscall_evt_lock);
    /* If we are called, syscall tracing is active*/
    if (syscall_evt == NULL) {
        syscall_evt = (QemuEventExec *)calloc(1, sizeof(QemuEventExec));
    }

    syscall_evt->syscall.num = num;
    syscall_evt->syscall.args[0] = a1;
    syscall_evt->syscall.args[1] = a2;
    syscall_evt->syscall.args[2] = a3;
    syscall_evt->syscall.args[3] = a4;
    syscall_evt->syscall.args[4] = a5;
    syscall_evt->syscall.args[5] = a6;
    syscall_evt->syscall.args[6] = a7;
    syscall_evt->syscall.args[7] = a8;
    g_mutex_unlock(&syscall_evt_lock);
}

static void callback_after_syscall(qemu_plugin_id_t id, unsigned int vcpu_idx,
                                   int64_t num, int64_t ret) {

    /* If we are called, syscall tracing is active */
    g_mutex_lock(&syscall_evt_lock);
    if (syscall_evt == NULL) {
        syscall_evt = (QemuEventExec *)calloc(1, sizeof(QemuEventExec));
    } else if (syscall_evt->syscall.num != num) {
        log_error("Syscall number mismatch: %d != %d", syscall_evt->syscall.num, num);
        free(syscall_evt);
        syscall_evt = NULL;
        g_mutex_unlock(&syscall_evt_lock);
        return;
    }

    SETSYSCALLS(syscall_evt->flags);
    syscall_evt->syscall.rv = ret;

    submit(sender, syscall_evt);

    free(syscall_evt);
    syscall_evt = NULL;
    g_mutex_unlock(&syscall_evt_lock);
}

ErrorCode callback_init(qemu_plugin_id_t id, bool trace_pc, bool trace_read,
                        bool trace_write, bool trace_instr, bool trace_syscall,
                        bool trace_branch, const char *socket_path) {

    SETFLAGS(flags, trace_pc, trace_read | trace_write, trace_instr, trace_syscall,
             trace_branch);

    g_mutex_lock(&exec_htable_lock);
    if ((exec_htable = g_hash_table_new(NULL, NULL)) == NULL) {
        log_error("Failed to create hash table for exec events");
        g_mutex_unlock(&exec_htable_lock);
        return OutOfMemory;
    }

    g_mutex_unlock(&exec_htable_lock);

    // Sadly, we can't pass the settings into everything.
    // Precompute whether we only need to instrument the last insn of a block
    if ((sender = setup(BATCH_SIZE, socket_path)) == NULL) {
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

    log_info("Initialized plugin callbacks.\n");

    return Success;
}
