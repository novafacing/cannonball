#ifndef ERROR_H
#define ERROR_H

// Definitions for errors that may occur during the plugin runtime
// These error codes are defined in no particular order except that they are added as
// needed during development.
typedef enum ErrorCode {
    // General success code
    Success = 0,
    // General failure code -- used when no other error code is appropriate
    Failure = 1,
    // Error code for when the plugin is loaded in system emulation mode (which is not
    // yet supported)
    SystemEmulationUnsupported = 2,
    // The directory the log file is specified to output to does not exist (error
    // because
    // we do not create the directory if it does not exist)
    MissingLogDirectory = 3,
    // The log file path is otherwise invalid (e.g. the path is a directory, the path is
    // empty, etc.)
    InvalidLogFilePath = 4,
    // The log file could not be opened for writing
    LogFileOpenFailed = 5,
    // Out of memory
    OutOfMemory = 6,
    // General argument parsing error, also returned when the `help` argument is used
    ArgumentErrorOrHelp = 7,
} ErrorCode;

#endif // ERROR_H