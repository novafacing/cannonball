#include <stddef.h>
#include <stdlib.h>

#include "cleanup.h"

// Double capacity on exhaustion
#define CAPACITY_BUFFER (2)
#define INITIAL_CAPACITY (8)

static FreeWrapperContainer *container = NULL;

static void cleanup_callback(qemu_plugin_id_t id, void *userdata) {
    FreeWrapperContainer *fwc = (FreeWrapperContainer *)userdata;

    for (size_t i = 0; i < fwc->num_wrappers; i++) {
        fwc->wrappers[i].wrapper(fwc->wrappers[i].obj);
    }
}

void cleanup_init(qemu_plugin_id_t id) {
    container = (FreeWrapperContainer *)calloc(1, sizeof(FreeWrapperContainer));
    qemu_plugin_register_atexit_cb(id, cleanup_callback, container);
}

void cleanup_add_wrapper(void (*wrapper)(void *), void *obj) {
    if (container->capacity == 0) {
        container->capacity = INITIAL_CAPACITY;
        container->wrappers =
            (FreeWrapper *)calloc(container->capacity, sizeof(FreeWrapper));
        container->num_wrappers = 0;
    }

    if (container->num_wrappers >= container->capacity) {
        container->capacity *= CAPACITY_BUFFER;
        container->wrappers = (FreeWrapper *)realloc(
            container->wrappers, container->capacity * sizeof(FreeWrapper));
    }

    FreeWrapper *fw = &container->wrappers[container->num_wrappers++];
    fw->wrapper = wrapper;
    fw->obj = obj;
}