# Plugin API

## Table of Contents

## Overview

The QEMU plugin API documentation is pretty hard to read so I've copied it down here.

## Setup and required exports

### QEMU_PLUGIN_VERSION

Plugins must export an `int qemu_plugin_version` to inform QEMU what their API
compatibility is, so probably all plugins should have:

```c
QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;
```

### qemu_plugin_install

Plugins won't be installed

## Gotchas and weird things

There are a few (or a lot...see below for how long this list gets) things that are
definitely "gotchas" that a new QEMU plugin developer may not be aware of. Here they
are, yipee!

### qemu_plugin_outs doesn't work

I don't know if this is just me or if it actually doesn't work, but using
`qemu_plugin_outs` for printouts seems to just straight up not work. YMMV of course, but
it isn't because your code is wrong if it isn't working for you either.

## Types

* `void (*qemu_plugin_simple_cb_t)(qemu_plugin_id_t id);`
* `void (*qemu_plugin_udata_cb_t)(qemu_plugin_id_t id, void *userdata);`
* `void (*qemu_plugin_vcpu_simple_cb_t)(qemu_plugin_id_t id, unsigned int vcpu_index);`
* `void (*qemu_plugin_vcpu_udata_cb_t)(unsigned int vcpu_index, void *userdata);`
* `void (*qemu_plugin_vcpu_mem_cb_t)(unsigned int vcpu_index, qemu_plugin_meminfo_t info, uint64_t vaddr, void *userdata);`
* `void (*qemu_plugin_vcpu_syscall_cb_t)(qemu_plugin_id_t id, unsigned int vcpu_index, int64_t num, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7, uint64_t a8); `
* `void (*qemu_plugin_vcpu_syscall_ret_cb_t)(qemu_plugin_id_t id, unsigned int vcpu_idx, int64_t num, int64_t ret);`

## API

### Non-Callback API

* `QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id, const qemu_info_t *info, int argc, char **argv);`
* `void qemu_plugin_uninstall(qemu_plugin_id_t id, qemu_plugin_simple_cb_t cb);`
* `void qemu_plugin_reset(qemu_plugin_id_t id, qemu_plugin_simple_cb_t cb);`
* `void (*qemu_plugin_vcpu_tb_trans_cb_t)(qemu_plugin_id_t id, struct qemu_plugin_tb *tb);`
* `size_t qemu_plugin_tb_n_insns(const struct qemu_plugin_tb *tb);`
* `uint64_t qemu_plugin_tb_vaddr(const struct qemu_plugin_tb *tb);`
* `struct qemu_plugin_insn * qemu_plugin_tb_get_insn(const struct qemu_plugin_tb *tb, size_t idx);`
* `const void *qemu_plugin_insn_data(const struct qemu_plugin_insn *insn);`
* `size_t qemu_plugin_insn_size(const struct qemu_plugin_insn *insn);`
* `uint64_t qemu_plugin_insn_vaddr(const struct qemu_plugin_insn *insn);`
* `void *qemu_plugin_insn_haddr(const struct qemu_plugin_insn *insn);`
* `unsigned int qemu_plugin_mem_size_shift(qemu_plugin_meminfo_t info);`
* `bool qemu_plugin_mem_is_sign_extended(qemu_plugin_meminfo_t info);`
* `bool qemu_plugin_mem_is_big_endian(qemu_plugin_meminfo_t info);`
* `bool qemu_plugin_mem_is_store(qemu_plugin_meminfo_t info);`
* `struct qemu_plugin_hwaddr *qemu_plugin_get_hwaddr(qemu_plugin_meminfo_t info, uint64_t vaddr);`
* `bool qemu_plugin_hwaddr_is_io(const struct qemu_plugin_hwaddr *haddr);`
* `uint64_t qemu_plugin_hwaddr_phys_addr(const struct qemu_plugin_hwaddr *haddr);`
* `const char *qemu_plugin_hwaddr_device_name(const struct qemu_plugin_hwaddr *h);`
* `char *qemu_plugin_insn_disas(const struct qemu_plugin_insn *insn);`
* `const char *qemu_plugin_insn_symbol(const struct qemu_plugin_insn *insn);`
* `void qemu_plugin_vcpu_for_each(qemu_plugin_id_t id, qemu_plugin_vcpu_simple_cb_t cb);`
* `int qemu_plugin_n_vcpus(void);`
* `int qemu_plugin_n_max_vcpus(void);`
* `void qemu_plugin_outs(const char *string);`
* `bool qemu_plugin_bool_parse(const char *name, const char *val, bool *ret);`
* `const char *qemu_plugin_path_to_binary(void);`
* `uint64_t qemu_plugin_start_code(void);`
* `uint64_t qemu_plugin_end_code(void);`
* `uint64_t qemu_plugin_entry_code(void);`

### Callback API

* `void qemu_plugin_register_vcpu_init_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_simple_cb_t cb);`
* `void qemu_plugin_register_vcpu_exit_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_simple_cb_t cb);`
* `void qemu_plugin_register_vcpu_idle_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_simple_cb_t cb);`
* `void qemu_plugin_register_vcpu_resume_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_simple_cb_t cb);`
* `void qemu_plugin_register_vcpu_tb_trans_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_tb_trans_cb_t cb);`
* `void qemu_plugin_register_vcpu_tb_exec_cb(struct qemu_plugin_tb *tb, qemu_plugin_vcpu_udata_cb_t cb, enum qemu_plugin_cb_flags flags, void *userdata);`
* `void qemu_plugin_register_vcpu_tb_exec_inline(struct qemu_plugin_tb *tb, enum qemu_plugin_op op, void *ptr, uint64_t imm);`
* `void qemu_plugin_register_vcpu_insn_exec_cb(struct qemu_plugin_insn *insn, qemu_plugin_vcpu_udata_cb_t cb, enum qemu_plugin_cb_flags flags, void *userdata);`
* `void qemu_plugin_register_vcpu_insn_exec_inline(struct qemu_plugin_insn *insn, enum qemu_plugin_op op, void *ptr, uint64_t imm);`
* `void qemu_plugin_register_vcpu_mem_cb(struct qemu_plugin_insn *insn, qemu_plugin_vcpu_mem_cb_t cb, enum qemu_plugin_cb_flags flags, enum qemu_plugin_mem_rw rw, void *userdata);`
* `void qemu_plugin_register_vcpu_mem_inline(struct qemu_plugin_insn *insn, enum qemu_plugin_mem_rw rw, enum qemu_plugin_op op, void *ptr, uint64_t imm);`
* `void qemu_plugin_register_vcpu_syscall_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_syscall_cb_t cb);`
* `void qemu_plugin_register_vcpu_syscall_ret_cb(qemu_plugin_id_t id, qemu_plugin_vcpu_syscall_ret_cb_t cb);`
* `void qemu_plugin_register_atexit_cb(qemu_plugin_id_t id, qemu_plugin_udata_cb_t cb, void *userdata); `
* `void qemu_plugin_register_flush_cb(qemu_plugin_id_t id, qemu_plugin_simple_cb_t cb);`