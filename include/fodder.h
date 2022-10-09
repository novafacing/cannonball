#ifndef FODDER_H

// Standard includes
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// QEMU includes
#include <qemu-plugin.h>

// Glib/GObject includes
#include <glib.h>

// Logger macros for DEBUG, INFO, WARN, ERROR, and FATAL
#define DEBUG(...) qemu_plugin_outs("[FDR DEBUG]: " __VA_ARGS__)
#define INFO(...) qemu_plugin_outs("[FDR  INFO]: " __VA_ARGS__)
#define WARN(...) qemu_plugin_outs("[FDR  WARN]: " __VA_ARGS__)
#define ERROR(...) qemu_plugin_outs("[FDR ERROR]: " __VA_ARGS__)
#define FATAL(...) qemu_plugin_outs("[FDR FATAL]: " __VA_ARGS__)

#define FREE(ptr)                                                                      \
    do {                                                                               \
        if (ptr) {                                                                     \
            free(ptr);                                                                 \
            ptr = NULL;                                                                \
        } else {                                                                       \
            WARN("Trying to free NULL pointer ##ptr");                                 \
        }                                                                              \
    } while (0)

#define INSTALL_SUCCESS (0)
#define INSTALL_FAILURE (1)

typedef struct {
    char *name;
} fodder_ctx_t;

/* Global context */
extern fodder_ctx_t *fodder_ctx;

/* Callback function declarations */
void fodder_onexit_cb(qemu_plugin_id_t, void *);
void fodder_ontrans_cb(qemu_plugin_id_t, struct qemu_plugin_tb *);

fodder_ctx_t *fodder_new(qemu_plugin_id_t, const qemu_info_t *);
void fodder_delete(fodder_ctx_t *);

#endif // FODDER_H