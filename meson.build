project('fodder', 'c')

libglib = dependency('glib-2.0', required: true)
libgobject = dependency('gobject-2.0', required: true)

qemu_root = get_option('qemu_root')
qemu_includes = qemu_root / 'include' / 'qemu'
qemu_inc_dir = include_directories(qemu_includes)

inc_dir = include_directories('include')

src_files = [
    'src/args.c',
    'src/cleanup.c',
    'src/install.c',
    'src/logging.c',
]

libfodder = shared_module(
    'fodder',
    src_files,
    include_directories: [inc_dir, qemu_inc_dir],
    dependencies: [libglib, libgobject],
)
