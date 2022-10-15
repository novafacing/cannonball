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

    ErrorCode rv = Success;

    cleanup_init(id);

    if ((rv = args_parse(argc, argv)) != Success) {
        // We never want to error out of qemu plugin install, otherwise our cleanup
        // code won't run
        return Success;
    }

    if ((rv = log_init(args_get()->log_file)) != Success) {
        // We never want to error out of qemu plugin install, otherwise our cleanup
        // code won't run
        return Success;
    }

    return Success;
}