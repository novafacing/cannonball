#ifndef ARGS_H
#define ARGS_H

#include <stdbool.h>
#include <stddef.h>
#include <sys/types.h>

#include "error.h"

// Argument definitions for to the plugin

#define HANDLER_EXIT (false)
#define HANDLER_CONTINUE (true)

typedef enum ArgType {
    Boolean,
    LongLong,
    String,
} ArgType;

typedef struct Arg {
    // The name of the argument
    const char *name;
    // The type of the argument, only Boolean, Integer, and String are supported
    ArgType type;
    // Whether the argument is required, if false, the argument is optional and a
    // default will be used
    bool required;
    // The default value for the argument, if required is false. If required is true,
    // this value should be NULL and will be ignored
    const char *default_value;
    // The description of the argument, used for generating help text
    const char *help;
    // The entry in the args struct for the argument, or -1 if there is no entry
    ssize_t entry;
    // A handler to call if the argument is seen on the command line, for example for
    // a help dialog. IF the handler returns false, execution will stop and the plugin
    // will not be loaded. If the handler returns true, execution will continue.
    bool (*handler)(void);
} Arg;

// Argument definitions for to the plugin

typedef struct Args {
    char *log_file;
    bool *trace_pc;
} Args;

// Parse arguments to the plugin. Arguments are passed in via the QEMU command line
// like: -plugin libplugin.so,arg1=val1,arg2=val2
//
ErrorCode args_parse(int argc, char **argv);

const Args *args_get(void);

#endif // ARGS_H