project('cannonball', 'c')

cc = meson.get_compiler('c')
libglib = dependency('glib-2.0', required: true)
libgobject = dependency('gobject-2.0', required: true)

subdir('cannonball-client')

qemu_root = get_option('qemu_root')

if qemu_root == ''
    qemu_subproj = subproject('qemu')
    qemu_inc_dir = qemu_subproj.get_variable('qemu_inc_dir')
    qemu_bins = qemu_subproj.get_variable('target_qemu_bins')
else
    qemu_includes = qemu_root / 'include' / 'qemu'
    qemu_inc_dir = include_directories(qemu_includes)
    qemu_bins = []
endif

inc_dir = include_directories('include')

src_files = [
    'src/args.c',
    'src/callback.c',
    'src/install.c',
    'src/logging.c',
    libcannonball_header,
]

libcannonball = shared_module(
    'cannonball',
    src_files,
    include_directories: [inc_dir, qemu_inc_dir],
    dependencies: [libglib, libgobject],
    link_with: [libcannonball_lib]
)

subdir('cannonball-tools')
