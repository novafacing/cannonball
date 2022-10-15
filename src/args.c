#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "args.h"
#include "cleanup.h"

// Parsing for command line arguments to the plugin

static Args *args = NULL;

static bool print_help(void);
static bool debug_args(void);

static Arg options[] = {
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
        .default_value = "-",
        .help = "Path to log file. If not specified, logs to stderr.",
        .entry = offsetof(Args, log_file),
        .handler = NULL,
    },
    {
        .name = "trace_pc",
        .type = Boolean,
        .required = false,
        .default_value = "true",
        .help = "Enable program counter tracing. Defaults to true.",
        .entry = offsetof(Args, trace_pc),
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

static bool print_help(void) {
    for (size_t i = 0; i < sizeof(options) / sizeof(Arg); i++) {
        Arg *arg = &options[i];
        printf("%12s=", arg->name);
        switch (arg->type) {
            case Boolean:
                printf("<boolean>");
                break;
            case String:
                printf("<string >");
                break;
            case Integer:
                printf("<integer>");
                break;
            default:
                printf("<unknown>");
                break;
        }
        if (arg->default_value != NULL) {
            switch (arg->type) {
                case Boolean:
                    printf(" (default: %5s)", arg->default_value);
                    break;
                case String:
                    printf(" (default: %5s)", arg->default_value);
                    break;
                case Integer:
                    printf(" (default: %5s)", arg->default_value);
                    break;
                default:
                    break;
            }
        }
        printf(" %s\n", arg->help);
    }
    return HANDLER_EXIT;
}

#ifndef RELEASE
static bool debug_args(void) {
    fprintf(stderr, "debug args:\n");
    fprintf(stderr, "log_file: %s\n", args->log_file);
    fprintf(stderr, "trace_pc: %d\n", *args->trace_pc);
    return HANDLER_EXIT;
}
#endif

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

static char **split_arg(char *arg) {
    char **split = NULL;
    char *token = NULL;
    char *saveptr = NULL;
    // strtok_r modifies the original string so we need to copy it
    char *to_parse = strdup(arg);

    if ((split = calloc(3, sizeof(char *))) == NULL) {
        fprintf(stderr, "Failed to allocate memory for split arg: %s\n",
                strerror(errno));
        goto err;
    }

    if ((token = strtok_r(to_parse, "=", &saveptr)) == NULL) {
        fprintf(stderr, "Failed to parse arg %s: %s\n", arg, strerror(errno));
        goto err;
    } else {
        split[0] = strdup(token);
    }

    if ((token = strtok_r(NULL, "=", &saveptr)) == NULL) {
        fprintf(stderr, "Failed to parse val %s: %s\n", arg, strerror(errno));
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

    fprintf(stderr, "Invalid boolean value: %s\n", val);
    return NULL;
}

static void free_args(void *obj) {
    Args *to_free = (Args *)obj;
    if (!to_free) {
        return;
    }

    if (to_free->log_file) {
        free(to_free->log_file);
    }

    if (to_free->trace_pc) {
        free(to_free->trace_pc);
    }

    free(to_free);
}

Args *parse_args(int argc, char **argv) {
    // Full argument value
    char *fullarg = NULL;
    // The argument name part of the full argument value
    const char *arg = NULL;
    // The argument value part of the full argument value
    const char *val = NULL;
    // Tokens from a split argument of the form: arg1=val1
    char **tokens = NULL;
    // The pointer to a new int arg value
    int *intarg = NULL;
    // The pointer to a new bool arg value
    bool *boolarg = NULL;
    // The pointer to a new string arg value
    char *strarg = NULL;
    // The current option being checked
    const Arg *option = NULL;
    // Whether the current option was seen
    bool opt_seen = false;

    args = (Args *)calloc(1, sizeof(Args));

    if (args == NULL) {
        fprintf(stderr, "Failed to allocate memory for args: %s\n", strerror(errno));
        return NULL;
    }

    for (size_t j = 0; j < sizeof(options) / sizeof(Arg); j++) {
        option = &options[j];
        opt_seen = false;

        for (size_t i = 0; i < argc; i++) {
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
                        return NULL;
                    }
                    free_arg(tokens);
                    continue;
                }

                switch (option->type) {
                    case Boolean:
                        boolarg = parse_bool(val);
                        if (boolarg == NULL) {
                            free_arg(tokens);
                            return NULL;
                        }
                        *((bool **)((char *)args + option->entry)) = parse_bool(val);
                        boolarg = NULL;
                        break;
                    case String:
                        strarg = strdup(val);
                        if (strarg == NULL) {
                            fprintf(stderr,
                                    "Failed to allocate memory for string arg: %s\n",
                                    strerror(errno));
                            return NULL;
                        }
                        *((char **)((char *)args + option->entry)) = strarg;
                        strarg = NULL;
                        break;
                    case Integer:
                        intarg = (int *)calloc(1, sizeof(int));
                        if (intarg == NULL) {
                            fprintf(stderr,
                                    "Failed to allocate memory for int arg: %s\n",
                                    strerror(errno));
                            return NULL;
                        }
                        *intarg = atoi(val);
                        *((int **)((char *)args + option->entry)) = intarg;
                        intarg = NULL;
                        break;
                    default:
                        fprintf(stderr, "Unknown option type: %d\n", option->type);
                        free_arg(tokens);
                        return NULL;
                }

                opt_seen = true;
            }

            // TODO: Once the above TODO is done, we can free the tokens at the end
            // instead of here
            free_arg(tokens);
        }

        if (!opt_seen && option->required) {
            fprintf(stderr, "Missing required option: %s\n", option->name);
            return NULL;
        }

        if (!opt_seen && !option->required && option->default_value) {
            val = option->default_value;

            switch (option->type) {
                case Boolean:
                    boolarg = parse_bool(val);
                    if (boolarg == NULL) {
                        return NULL;
                    }
                    *((bool **)((char *)args + option->entry)) = parse_bool(val);
                    boolarg = NULL;
                    break;
                case String:
                    strarg = strdup(val);
                    if (strarg == NULL) {
                        fprintf(stderr,
                                "Failed to allocate memory for string arg: %s\n",
                                strerror(errno));
                        return NULL;
                    }
                    *((char **)((char *)args + option->entry)) = strarg;
                    strarg = NULL;
                    break;
                case Integer:
                    intarg = (int *)calloc(1, sizeof(int));
                    if (intarg == NULL) {
                        fprintf(stderr, "Failed to allocate memory for int arg: %s\n",
                                strerror(errno));
                        return NULL;
                    }
                    *intarg = atoi(val);
                    *((int **)((char *)args + option->entry)) = intarg;
                    intarg = NULL;
                    break;
                default:
                    fprintf(stderr, "Unknown option type: %d\n", option->type);
                    return NULL;
            }
        }
    }

    cleanup_add_wrapper(free_args, args);
    return args;
}
