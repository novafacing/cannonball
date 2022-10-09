#include "qemu-plugin.h"
#include <fodder.h>
#include <stdio.h>

fodder_ctx_t *fodder_ctx = NULL;

/* Called on plugin exit
 *
 * @id - plugin id
 * @p - plugin handle
 */

void fodder_onexit_cb(qemu_plugin_id_t id, void *p) {
    fodder_ctx_t *fodder_ctx = (fodder_ctx_t *)p;
    fodder_delete(fodder_ctx);
}

void fodder_ontrans_cb(qemu_plugin_id_t id, struct qemu_plugin_tb *tb) {
    size_t tb_size = qemu_plugin_tb_n_insns(tb);
    char pcbuf[0x100] = {0};
    qemu_plugin_outs("Translating...");

    for (size_t i = 0; i < tb_size; i++) {
        struct qemu_plugin_insn *insn = qemu_plugin_tb_get_insn(tb, i);
        uint64_t pc = qemu_plugin_insn_vaddr(insn);
        int bsz = snprintf(pcbuf, sizeof(pcbuf) - 1, "0x%lx", pc);
        qemu_plugin_outs(pcbuf);
        memset(pcbuf, 0, bsz);
    }
}

/**
 * @brief Initialize fodder plugin context
 *
 * @param id - plugin id
 * @param info - qemu info
 *
 * @return fodder_ctx_t* - pointer to fodder context
 */
fodder_ctx_t *fodder_new(qemu_plugin_id_t id, const qemu_info_t *info) {
    fodder_ctx_t *fodder_ctx = malloc(sizeof(fodder_ctx_t));
    fodder_ctx->name = strdup(info->target_name);
    return fodder_ctx;
}

/**
 * @brief Free fodder plugin context
 *
 * @param fodder_ctx - pointer to fodder context
 */
void fodder_delete(fodder_ctx_t *fodder_ctx) {
    FREE(fodder_ctx->name);
    FREE(fodder_ctx);
}