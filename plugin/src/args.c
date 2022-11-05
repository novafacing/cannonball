#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "args.h"
#include "error.h"
#include "logging.h"

/// Parsing for command line arguments to the plugin

static Args *args = NULL;

static bool print_help(void);
static bool debug_args(void);

/// Command-line configuration options for the plugin
static const Arg options[] = {
    {
        .name = "help",
        .type = Boolean,
        .required = false,
        .default_value = NULL,
        .help = "Print this help message",
        .entry = -1,
        .handler = print_help,
    },
    {
        .name = "log_file",
        .type = String,
        .required = false,
        .default_value = "-", // NOTE: "-" is interpreted as stderr NOT stdout -- only
                              // the binary should print to stdout
        .help = "Path to log file. '-' is interpreted as stderr.",
        .entry = offsetof(Args, log_file),
        .handler = NULL,
    },
    {
        .name = "log_level",
        .type = LongLong,
        .required = false,
        .default_value = "3",
        .help = "Log level (0 = Disabled, 1 = Error, 2 = Warning, 3 = Info, 4 = Debug)",
        .entry = offsetof(Args, log_level),
        .handler = NULL,
    },
    {
        .name = "sock_path",
        .type = String,
        .required = false,
        .default_value = "/dev/shm/cannonball.sock",
        .help = "Path to socket file to connect to consumer.",
        .entry = offsetof(Args, sock_path),
        .handler = NULL,
    },
    {
        .name = "trace_pc",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable program counter tracing.",
        .entry = offsetof(Args, trace_pc),
        .handler = NULL,
    },
    {
        .name = "trace_reads",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable memory read tracing.",
        .entry = offsetof(Args, trace_reads),
        .handler = NULL,
    },
    {
        .name = "trace_writes",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable memory write tracing.",
        .entry = offsetof(Args, trace_writes),
        .handler = NULL,
    },
    {
        .name = "trace_syscalls",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable syscall tracing.",
        .entry = offsetof(Args, trace_syscalls),
        .handler = NULL,
    },
    {
        .name = "trace_instrs",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable instruction contents tracing.",
        .entry = offsetof(Args, trace_instrs),
        .handler = NULL,
    },
    {
        .name = "trace_branches",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable branch tracing.",
        .entry = offsetof(Args, trace_branches),
        .handler = NULL,
    },
#ifndef RELEASE
    {
        .name = "debug_args",
        .type = Boolean,
        .required = false,
        .default_value = "false",
        .help = "Enable debugging of program arguments for development purposes.",
        .entry = -1,
        .handler = debug_args,
    },
#endif
};

/// Print out the help message and signal an exit (if you need help, you probably don't
/// want to run, I figure)
static bool print_help(void) {
    for (size_t i = 0; i < sizeof(options) / sizeof(Arg); i++) {
        const Arg *arg = &options[i];
        printf("%16s=", arg->name);
        switch (arg->type) {
            case Boolean:
                printf("<boolean>");
                break;
            case String:
                printf("<string >");
                break;
            case LongLong:
                printf("<integer>");
                break;
            default:
                printf("<unknown>");
                break;
        }
        printf(" %s\n", arg->help);
        if (arg->default_value != NULL) {
            printf("                           "
                   "(default: %s)\n",
                   arg->default_value);
        }
        printf("\n");
    }
    return HANDLER_EXIT;
}

#ifndef RELEASE
/// Print out the arguments and signal an exit. This is just for development purposes
/// and debugging
static bool debug_args(void) {
    log_debug("debug args:\n");
    log_debug("    log_file:       %s\n", args->log_file);
    log_debug("    log_level:      %lld", *args->log_level);
    log_debug("    sock_path:      %s\n", args->sock_path);
    log_debug("    trace_pc:       %d\n", *args->trace_pc);
    log_debug("    trace_reads:    %d\n", *args->trace_reads);
    log_debug("    trace_writes:   %d\n", *args->trace_writes);
    log_debug("    trace_syscalls: %d\n", *args->trace_syscalls);
    log_debug("    trace_instrs:   %d\n", *args->trace_instrs);
    log_debug("    trace_branches: %d\n", *args->trace_branches);
    return HANDLER_EXIT;
}
#endif

/// Free up the resources for a split argument (ex 'arg1=val1')
static void free_arg(char **arg) {
    if (arg != NULL) {
        if (arg[0] != NULL) {
            free(arg[0]);
        }
        if (arg[1] != NULL) {
            free(arg[1]);
        }
        free(arg);
    }
}

/// Split an argument into a key and value (ex 'arg1=val1' -> ['arg1', 'val1'])
static char **split_arg(char *arg) {
    char **split = NULL;
    char *token = NULL;
    char *saveptr = NULL;
    // strtok_r modifies the original string so we need to copy it
    char *to_parse = strdup(arg);

    if ((split = calloc(3, sizeof(char *))) == NULL) {
        log_error("Failed to allocate memory for split arg: %s\n", strerror(errno));
        goto err;
    }

    if ((token = strtok_r(to_parse, "=", &saveptr)) == NULL) {
        log_error("Failed to parse arg %s: %s\n", arg, strerror(errno));
        goto err;
    } else {
        split[0] = strdup(token);
    }

    if ((token = strtok_r(NULL, "=", &saveptr)) == NULL) {
        log_error("Failed to parse val %s: %s\n", arg, strerror(errno));
        goto err;
    } else {
        split[1] = strdup(token);
    }

    free(to_parse);
    return split;

err:
    free(to_parse);
    free_arg(split);
    return NULL;
}

/// Parse a boolean argument from a string (ex 'true', 'yes', '1', 'on' -> true)
static bool *parse_bool(const char *val) {
    const char *true_vals[] = {"true", "yes", "1", "on"};
    const char *false_vals[] = {"false", "no", "0", "off"};
    bool *rv = (bool *)calloc(1, sizeof(bool));

    for (size_t i = 0; i < sizeof(true_vals) / sizeof(true_vals[0]); i++) {
        if (strcmp(val, true_vals[i]) == 0) {
            *rv = true;
            return rv;
        }
    }

    for (size_t i = 0; i < sizeof(false_vals) / sizeof(false_vals[0]); i++) {
        if (strcmp(val, false_vals[i]) == 0) {
            *rv = false;
            return rv;
        }
    }

    log_error("Invalid boolean value: %s\n", val);
    free(rv);
    return NULL;
}

/// Free up the resources for the global args struct
void args_free(void) {
    if (!args) {
        return;
    }

    if (args->log_file) {
        free(args->log_file);
        args->log_file = NULL;
    }

    if (args->log_level) {
        free(args->log_level);
        args->log_level = NULL;
    }

    if (args->sock_path) {
        free(args->sock_path);
        args->sock_path = NULL;
    }

    if (args->trace_pc) {
        free(args->trace_pc);
        args->trace_pc = NULL;
    }

    if (args->trace_reads) {
        free(args->trace_reads);
        args->trace_reads = NULL;
    }

    if (args->trace_writes) {
        free(args->trace_writes);
        args->trace_writes = NULL;
    }

    if (args->trace_syscalls) {
        free(args->trace_syscalls);
        args->trace_syscalls = NULL;
    }

    if (args->trace_instrs) {
        free(args->trace_instrs);
        args->trace_instrs = NULL;
    }

    if (args->trace_branches) {
        free(args->trace_branches);
        args->trace_branches = NULL;
    }

    free(args);
    args = NULL;
}

/// Parse the command line arguments from argc and argv
ErrorCode args_parse(int argc, char **argv) {
    /// Full argument value
    char *fullarg = NULL;
    /// The argument name part of the full argument value
    const char *arg = NULL;
    /// The argument value part of the full argument value
    const char *val = NULL;
    /// Tokens from a split argument of the form: arg1=val1
    char **tokens = NULL;
    /// The pointer to a new int arg value
    long long int *intarg = NULL;
    /// The pointer to a new bool arg value
    bool *boolarg = NULL;
    /// The pointer to a new string arg value
    char *strarg = NULL;
    /// The current option being checked
    const Arg *option = NULL;
    /// Whether the current option was seen
    bool opt_seen = false;

    args = (Args *)calloc(1, sizeof(Args));

    if (args == NULL) {
        log_error("Failed to allocate memory for args: %s\n", strerror(errno));
        return OutOfMemory;
    }

    // TODO: This is a little inefficient, ostensibly we would iterate the outer loop
    // over the args so we don't need to alloc and free as many times, but it really
    // isn't a big deal since this only happens once
    for (size_t j = 0; j < sizeof(options) / sizeof(Arg); j++) {
        option = &options[j];
        opt_seen = false;

        for (int i = 0; i < argc; i++) {
            fullarg = argv[i];

            // TODO: This should cache the split args so we only have to split them once
            if (!fullarg || (tokens = split_arg(fullarg)) == NULL) {
                continue;
            }

            arg = tokens[0];
            val = tokens[1];

            // Check each option
            if (strcmp(arg, option->name) == 0) {
                if (option->handler != NULL) {
                    if (!option->handler()) {
                        free_arg(tokens);
                        return ArgumentHandlerExit;
                    }
                    free_arg(tokens);
                    continue;
                }

                // TODO: make this switch a function, it's duplicated below
                switch (option->type) {
                    case Boolean:
                        boolarg = parse_bool(val);
                        if (boolarg == NULL) {
                            free_arg(tokens);
                            return ArgumentError;
                        }
                        *((bool **)((char *)args + option->entry)) = parse_bool(val);
                        boolarg = NULL;
                        break;
                    case String:
                        strarg = strdup(val);
                        if (strarg == NULL) {
                            log_error("Failed to allocate memory for string arg: %s\n",
                                      strerror(errno));
                            return OutOfMemory;
                        }
                        *((char **)((char *)args + option->entry)) = strarg;
                        strarg = NULL;
                        break;
                    case LongLong:
                        intarg = (long long int *)calloc(1, sizeof(long long int));
                        if (intarg == NULL) {
                            log_error("Failed to allocate memory for int arg: %s\n",
                                      strerror(errno));
                            return OutOfMemory;
                        }
                        errno = 0;
                        *intarg = strtoll(val, NULL, 10);
                        if ((*intarg == LLONG_MAX || *intarg == LLONG_MIN ||
                             *intarg == 0) &&
                            errno != 0) {
                            log_error("Failed to parse llong from %s: %s\n", val,
                                      strerror(errno));
                            return ArgumentError;
                        }
                        *((long long int **)((char *)args + option->entry)) = intarg;
                        intarg = NULL;
                        break;
                    default:
                        log_error("Unknown option type: %d\n", option->type);
                        free_arg(tokens);
                        return ArgumentError;
                }

                opt_seen = true;
            }

            // TODO: Once the above TODO is done, we can free the tokens at the end
            // instead of here
            free_arg(tokens);
        }

        if (!opt_seen && option->required) {
            log_error("Missing required option: %s\n", option->name);
            return ArgumentError;
        }

        // Set default values for options that weren't seen
        if (!opt_seen && !option->required && option->default_value &&
            option->entry != -1) {
            val = option->default_value;

            switch (option->type) {
                case Boolean:
                    boolarg = parse_bool(val);
                    if (boolarg == NULL) {
                        return ArgumentError;
                    }
                    *((bool **)((char *)args + option->entry)) = parse_bool(val);
                    boolarg = NULL;
                    break;
                case String:
                    strarg = strdup(val);
                    if (strarg == NULL) {
                        log_error("Failed to allocate memory for string arg: %s\n",
                                  strerror(errno));
                        return OutOfMemory;
                    }
                    *((char **)((char *)args + option->entry)) = strarg;
                    strarg = NULL;
                    break;
                case LongLong:
                    intarg = (long long int *)calloc(1, sizeof(long long int));
                    if (intarg == NULL) {
                        log_error("Failed to allocate memory for int arg: %s\n",
                                  strerror(errno));
                        return OutOfMemory;
                    }
                    *intarg = atoi(val);
                    errno = 0;
                    *intarg = strtoll(val, NULL, 10);
                    if ((*intarg == LLONG_MAX || *intarg == LLONG_MIN ||
                         *intarg == 0) &&
                        errno != 0) {
                        log_error("Failed to parse llong from %s: %s\n", val,
                                  strerror(errno));
                        return ArgumentError;
                    }
                    *((long long int **)((char *)args + option->entry)) = intarg;
                    intarg = NULL;
                    break;
                default:
                    log_error("Unknown option type: %d\n", option->type);
                    return ArgumentError;
            }
        }
    }

    return Success;
}

const Args *args_get(void) { return args; }
