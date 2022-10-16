#ifndef ARGS_H
#define ARGS_H

#include <stdbool.h>
#include <stddef.h>
#include <sys/types.h>

#include "error.h"

// Argument definitions for to the plugin

/// Whether a handler exits after running or continues (for example, print_help is a
/// handler that exits after running)
#define HANDLER_EXIT (false)
#define HANDLER_CONTINUE (true)

/// The type of an argument, used to determine how to parse the argument
typedef enum ArgType {
    Boolean,
    LongLong,
    String,
} ArgType;

/// An argument, used to define the arguments that the plugin accepts
typedef struct Arg {
    /// The name of the argument
    const char *name;
    /// The type of the argument, only Boolean, Integer, and String are supported
    ArgType type;
    /// Whether the argument is required, if false, the argument is optional and a
    /// default will be used
    bool required;
    /// The default value for the argument, if required is false. If required is true,
    /// this value should be NULL and will be ignored
    const char *default_value;
    /// The description of the argument, used for generating help text
    const char *help;
    /// The entry in the args struct for the argument, or -1 if there is no entry
    ssize_t entry;
    /// A handler to call if the argument is seen on the command line, for example for
    /// a help dialog. IF the handler returns false, execution will stop and the plugin
    /// will not be loaded. If the handler returns true, execution will continue.
    bool (*handler)(void);
} Arg;

/// Argument definitions for to the plugin
typedef struct Args {
    /// The file name output will be logged to
    char *log_file;
    /// The log level to use
    long long int *log_level;
    /// The path to the unix socket the consumer is listening on
    char *sock_path;
    /// Whether we should trace program counters
    bool *trace_pc;
    /// Whether we should trace memory accesses
    bool *trace_reads;
    bool *trace_writes;
    /// Whether we should trace system calls
    bool *trace_syscalls;
    /// Whether we should trace instruction opcodes
    bool *trace_instrs;
    /// Whether we should trace branches
    bool *trace_branches;
} Args;

/// Parse arguments to the plugin. Arguments are passed in via the QEMU command line
/// like: -plugin libplugin.so,arg1=val1,arg2=val2
ErrorCode args_parse(int argc, char **argv);

/// Free the global argument resources
void args_free(void);

/// Return the global argument struct
const Args *args_get(void);

#endif // ARGS_H