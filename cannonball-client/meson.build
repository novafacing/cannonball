cannonball_client_inc_dir = include_directories('ffi')

libcannonball_client = custom_target(
    'libcannonball_client.a',
    output: 'libcannonball_client.a',
    input: ['src/client.rs', 'src/lib.rs', 'src/qemu_event.rs'],
    command: [
        meson.current_source_dir() / 'mesonbuild.sh',
        meson.current_build_dir(),
        'libcannonball_client.a'
    ]
)