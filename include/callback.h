#ifndef CALLBACK_H
#define CALLBACK_H

#include <qemu-plugin.h>

#include "error.h"

// Declarations for callback functions invoked by QEMU during the plugin runtime

ErrorCode callback_init(qemu_plugin_id_t id, bool trace_pc, bool trace_read,
                        bool trace_write, bool trace_instr, bool trace_syscall,
                        bool trace_branch, const char *socket_path);

// These callback types are only useful for system mode:

// void qemu_plugin_register_vcpu_init_cb(qemu_plugin_id_t id,
// qemu_plugin_vcpu_simple_cb_t cb);

// void qemu_plugin_register_vcpu_exit_cb(qemu_plugin_id_t id,
// qemu_plugin_vcpu_simple_cb_t cb);

// void qemu_plugin_register_vcpu_idle_cb(qemu_plugin_id_t id,
// qemu_plugin_vcpu_simple_cb_t cb);

// void qemu_plugin_register_vcpu_resume_cb(qemu_plugin_id_t id,
// qemu_plugin_vcpu_simple_cb_t cb);

#endif // CALLBACK_H