#ifndef CALLBACK_H
#define CALLBACK_H

#include <qemu-plugin.h>

#include "error.h"

/// Declarations for callback functions invoked by QEMU during the plugin runtime

/// Initialize the callbacks based on what trace events are requested. This function
/// also sets up the pipe to send the trace events to the consumer.
ErrorCode callback_init(qemu_plugin_id_t id, bool trace_pc, bool trace_read,
                        bool trace_write, bool trace_instr, bool trace_syscall,
                        bool trace_branch, const char *socket_path);
#endif // CALLBACK_H