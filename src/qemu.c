/* fodder plugin qemu interactions
 *
 * Code that handles direct interaction with qemu goes in here, and will hand off or
 * receive information from the fodder plugin.
 */

#include <fodder.h>

QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;

QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id, const qemu_info_t *info,
                                           int argc, char **argv) {

    bool is_system_emulation = info->system_emulation;

    if (is_system_emulation) {
        return INSTALL_FAILURE;
    }

    fodder_ctx = fodder_new(id, info);

    qemu_plugin_register_vcpu_tb_trans_cb(id, fodder_ontrans_cb);
    qemu_plugin_register_atexit_cb(id, fodder_onexit_cb, fodder_ctx);

    return INSTALL_SUCCESS;
}