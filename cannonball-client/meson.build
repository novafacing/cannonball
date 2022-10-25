libcannonball_client = custom_target(
    'libcannonball_client.a',
    output: ['libcannonball_client.a', 'cannonball-client.h'],
    input: ['src/client.rs', 'src/lib.rs', 'src/qemu_event.rs'],
    command: [
        meson.current_source_dir() / 'mesonbuild.sh',
        meson.current_build_dir(),
        '@OUTPUT@'
    ]
)
libcannonball_lib = libcannonball_client[0]
libcannonball_header = libcannonball_client[1]

# cannonball_client_inc_dir = include_directories('target' / 'ffi')