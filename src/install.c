/* cannonball plugin qemu interactions
 *
 * Code that handles direct interaction with qemu goes in here, and will hand off or
 * receive information from the cannonball plugin.
 */

#include <stdio.h>

#include "args.h"
#include "callback.h"
#include "config.h"
#include "error.h"
#include "install.h"
#include "logging.h"

QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;

QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id, const qemu_info_t *info,
                                           int argc, char **argv) {

    ErrorCode rv = Success;
    if ((rv = args_parse(argc, argv)) != Success) {
        goto cleanup;
    }

    const Args *args = args_get();

    if ((rv = log_init(args->log_file, (LogLevel)*args->log_level)) != Success) {
        goto cleanup;
    }

    if ((rv =
             callback_init(id, *args->trace_pc, *args->trace_reads, *args->trace_writes,
                           *args->trace_instrs, *args->trace_syscalls,
                           *args->trace_branches, args->sock_path)) != Success) {
        goto cleanup;
    }

cleanup:
    // TODO: Move this stuff into a cleanup function
    // args_free();
    // log_free();
    // instrumentation_settings_free((InstrumentationSettings *)settings);
    // settings = NULL;
    return rv;
}