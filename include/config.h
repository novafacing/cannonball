#ifndef CONFIG_H
#define CONFIG_H

// Configuration for the plugin system

// Generally, we're only going to support running on the platform we compile
// for...ostensibly none of this is Linux exclusive so we'll plan to support macos and
// Windows later
#define IS_WINDOWS (_WIN32 || _WIN64)
#define IS_MACOS (__APPLE__ && __MACH__)
#define IS_LINUX                                                                       \
    (__linux__ || __linux || linux || __gnu_linux__ || (!IS_WINDOWS && !IS_MACOS))

#if !IS_LINUX
#error "Unsupported platform, please file some PRs!"
#endif

#endif // CONFIG_H