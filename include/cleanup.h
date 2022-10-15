#ifndef CLEANUP_H
#define CLEANUP_H

#include <stddef.h>

#include <qemu-plugin.h>

// Cleanup functionality to sweep up at plugin exit or unload time

// A pair of free function, object where the free function will be called at plugin exit
// to clean up the object
typedef struct FreeWrapper {
    void (*wrapper)(void *);
    void *obj;
} FreeWrapper;

typedef struct FreeWrapperContainer {
    FreeWrapper *wrappers;
    size_t num_wrappers;
    size_t capacity;
} FreeWrapperContainer;

void cleanup_init(qemu_plugin_id_t id);
void cleanup_add_wrapper(void (*wrapper)(void *), void *obj);

#endif // CLEANUP_H