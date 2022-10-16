fodder_client_inc_dir = include_directories('ffi')

libfodder_client = custom_target(
    'libfodder_client.a',
    output: 'libfodder_client.a',
    input: ['src/client.rs', 'src/lib.rs', 'src/qemu_event.rs'],
    command: [
        meson.current_source_dir() / 'mesonbuild.sh',
        meson.current_build_dir(),
        'libfodder_client.a'
    ]
)