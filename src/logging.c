#include <errno.h>
#include <libgen.h>
#include <linux/limits.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "error.h"
#include "logging.h"

static char *log_file_path = NULL;
static FILE *log_file = NULL;
static LogLevel log_level = Debug;

static const char *level_strings[] = {
    [Error] = "ERROR",
    [Warning] = "WARN",
    [Info] = "INFO",
    [Debug] = "DEBUG",
};

static bool is_dir(const char *path) {
    struct stat path_stat;
    if (stat(path, &path_stat) != 0) {
        // No need to check errno -- none of the error conditions indicate a valid
        // directory
        return false;
    }
    return S_ISDIR(path_stat.st_mode);
}

void log_free(void) {
    if (log_file != NULL && log_file != stderr) {
        fclose(log_file);
        log_file = NULL;
    }

    // If we were unsuccessful, free the log file path
    if (log_file_path != NULL) {
        free((void *)log_file_path);
        log_file_path = NULL;
    }
}

ErrorCode log_init(const char *path) {
    ErrorCode rv = Success;
    char *log_file_dirpath = NULL;
    char *log_file_dir = NULL;
    size_t max_pathlen = PATH_MAX;

    if (path == NULL) {
        log_error("Log file path must not be null\n");
        rv = InvalidLogFilePath;
        goto cleanup;
    }

    if (strcmp(path, "-") == 0) {
        log_file = stderr;
        goto cleanup;
    }

    size_t log_file_path_len = strnlen(path, max_pathlen);

    if (log_file_path_len == 0) {
        log_error("Log file path must not be empty\n");
        rv = InvalidLogFilePath;
        goto cleanup;
    }

    // TODO: Ideally, we would check if the directory exists, but realpath checks if the
    // *file* exists so we can't use it to canonicalize. That's fine, but it is a little
    // less than perfect

    // if ((log_file_realpath = realpath(path, log_file_realpath)) ==
    // NULL) {
    //     log_error("Log file path is invalid: %s\n", strerror(errno));
    //     rv = InvalidLogFilePath;
    //     goto cleanup;
    // }
    if ((log_file_path = strdup(path)) == NULL) {
        log_error("Failed to copy log file path: %s\n", strerror(errno));
        rv = OutOfMemory;
        goto cleanup;
    }

    if (is_dir(log_file_path)) {
        log_error("Log file path must not be a directory\n");
        rv = InvalidLogFilePath;
        goto cleanup;
    }

    if ((log_file_dirpath = strdup(log_file_path)) == NULL) {
        log_error("Failed to copy log file path: %s\n", strerror(errno));
        rv = OutOfMemory;
        goto cleanup;
    }

    log_file_dir = dirname(log_file_dirpath);

    if (!is_dir(log_file_dir)) {
        log_error("Log file directory does not exist: %s\n", log_file_dir);
        rv = MissingLogDirectory;
        goto cleanup;
    }

    if ((log_file = fopen(log_file_path, "w")) == NULL) {
        log_error("Failed to open log file: %s\n", strerror(errno));
        rv = LogFileOpenFailed;
        goto cleanup;
    }

cleanup:
    // log_file_realpath is malloc()ed by realpath(), we
    // If we were unsuccessful, close the log file
    if (log_file_dirpath != NULL) {
        free((void *)log_file_dirpath);
        log_file_dirpath = NULL;
    }

    if (rv == Success) {
        log_info("Logging configured.\n");
    } else {
        log_free();
    }

    return rv;
}

void log_set_level(LogLevel level) { log_level = level; }

void log_message(LogLevel level, const char *format, va_list args) {
    FILE *outs = log_file;

    if (level > log_level) {
        return;
    }

    if (outs == NULL) {
        // If logging isn't initialized yet, or there was some logging error, we still
        // want to display messages
        outs = stderr;
    }

    fprintf(outs, "[%5s] ", level_strings[level]);

    vfprintf(outs, format, args);
}

void log_error(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_message(Error, format, args);
    va_end(args);
}

void log_warning(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_message(Warning, format, args);
    va_end(args);
}

void log_info(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_message(Info, format, args);
    va_end(args);
}

void log_debug(const char *format, ...) {
    va_list args;
    va_start(args, format);
    log_message(Debug, format, args);
    va_end(args);
}
