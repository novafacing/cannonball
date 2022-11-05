#ifndef LOGGING_H
#define LOGGING_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>

#include "error.h"

/// Log levels
typedef enum LogLevel {
    /// Log level to disable logging
    Disabled = 0,
    /// Log level for errors
    Error = 1,
    /// Log level for warnings
    Warning = 2,
    /// Log level for informational messages
    Info = 3,
    /// Log level for debugging messages
    Debug = 4,
} LogLevel;

/// Free the logging resources
void log_free(void);

/// Initialize the logging subsystem, which is used to output debug, info, warn,
/// error, and fatal messages. This function called during plugin intialization
/// and the log file path is checked to see if it is a valid path. If the containing
/// directory does not exist, it is NOT created an an error is returned.
ErrorCode log_init(const char *log_file_path, LogLevel level);

/// Set the log level, which controls what messages are output. Messages with a
/// log level less than or equal to the current log level will be output (where lower is
/// more serious with Error=0)
void log_set_level(LogLevel level);

/// Log a message at the given log level. The message is formatted using the
/// printf-style format string and arguments.
void log_message(LogLevel level, const char *format, va_list args);

/// Log a message at the error log level. The message is formatted using the
/// printf-style format string and arguments.
void log_error(const char *format, ...);

/// Log a message at the warning log level. The message is formatted using the
/// printf-style format string and arguments.
void log_warning(const char *format, ...);

/// Log a message at the info log level. The message is formatted using the
/// printf-style format string and arguments.
void log_info(const char *format, ...);

/// Log a message at the debug log level. The message is formatted using the
/// printf-style format string and arguments.
void log_debug(const char *format, ...);

#endif /// LOGGING_H