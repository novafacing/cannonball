/* fodder plugin qemu interactions
 *
 * Code that handles direct interaction with qemu goes in here, and will hand off or
 * receive information from the fodder plugin.
 */

#include <stdio.h>

#include "args.h"
#include "cleanup.h"
#include "config.h"
#include "error.h"
#include "install.h"
#include "logging.h"

QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;

QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id, const qemu_info_t *info,
                                           int argc, char **argv) {

    Args *args = NULL;
    ErrorCode rv = Success;

    cleanup_init(id);

    if ((args = args_parse(argc, argv)) == NULL) {
        rv = ArgumentErrorOrHelp;
        // No error message here because this can also be a help message
        goto cleanup;
    }

    if ((rv = log_init(args->log_file)) != Success) {
        goto cleanup;
    }

    log_info("Logging configured\n");

cleanup:
    return rv;
}