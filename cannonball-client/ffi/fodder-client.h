#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define MAX_OPCODE_SIZE 16

#define NUM_SYSCALL_ARGS 8

typedef struct Sender Sender;

typedef struct EventFlags {
  uint32_t bits;
} EventFlags;
#define EventFlags_PC (EventFlags){ .bits = (uint32_t)1 }
#define EventFlags_READS_WRITES (EventFlags){ .bits = (uint32_t)2 }
#define EventFlags_INSTRS (EventFlags){ .bits = (uint32_t)8 }
#define EventFlags_SYSCALLS (EventFlags){ .bits = (uint32_t)16 }
#define EventFlags_BRANCHES (EventFlags){ .bits = (uint32_t)32 }
#define EventFlags_EXECUTED (EventFlags){ .bits = (uint32_t)64 }

typedef struct QemuPc {
  uint64_t pc;
} QemuPc;

typedef struct QemuInstr {
  uint8_t opcode[MAX_OPCODE_SIZE];
  uintptr_t opcode_size;
} QemuInstr;

typedef struct QemuRead {
  /**
   * The virtual address of the read
   */
  uint64_t addr;
} QemuRead;

typedef struct QemuWrite {
  /**
   * The virtual address of the write
   */
  uint64_t addr;
} QemuWrite;

typedef struct QemuSyscall {
  /**
   * The syscall number that was executed
   */
  int64_t num;
  /**
   * The return value of the syscall
   */
  int64_t rv;
  /**
   * The syscall arguments (NOTE: any pointers are not visible)
   */
  uint64_t args[NUM_SYSCALL_ARGS];
} QemuSyscall;

typedef struct QemuBranch {
  bool branch;
} QemuBranch;

typedef struct QemuEventExec {
  struct EventFlags flags;
  /**
   * The program counter of the execution
   */
  struct QemuPc pc;
  struct QemuInstr instr;
  struct QemuRead read;
  struct QemuWrite write;
  struct QemuSyscall syscall;
  struct QemuBranch branch;
} QemuEventExec;

struct Sender *setup(uintptr_t batch_size, const char *socket);

void submit(struct Sender *client, struct QemuEventExec *event);

void teardown(struct Sender *_client);

void dbg_print_evt(struct QemuEventExec *event);
