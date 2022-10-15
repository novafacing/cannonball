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

// Dynamically resizable freewrapper container
typedef struct FreeWrapperContainer {
    FreeWrapper *wrappers;
    size_t num_wrappers;
    size_t capacity;
} FreeWrapperContainer;

// Setup cleanup system to deallocate long-lived memory at plugin exit.
// This should be used sparingly (for example it is used for deallocating
// program arguments))
void cleanup_init(qemu_plugin_id_t id);

// Add a wrapper to the cleanup process. If some object `obj` needs to be deallocated
// at plugin exit, add a wrapper to do so here.
void cleanup_add_wrapper(void (*wrapper)(void *), void *obj);

#endif // CLEANUP_H