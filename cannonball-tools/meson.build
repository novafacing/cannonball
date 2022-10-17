cannonball_tools = custom_target(
    'cannonball_tools',
    # Add your tool names here
    output: [
        'json'
    ],
    # Add your source files here (you don't have to, but for clarity)
    input: [
        'src/json/main.rs'
    ],
    command: [
        meson.current_source_dir() / 'mesonbuild.sh',
        meson.current_build_dir(),
        meson.build_root(),
        qemu_bins.to_list(),
    ],
    build_by_default: true
)