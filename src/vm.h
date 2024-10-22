#ifndef FVM_H
#define FVM_H

#include <stdint.h>

#include "hashmap.h"
#include "stack.h"

const uint8_t VERSION[3] = {0, 0, 0};
const uint8_t MAGIC_NUMBER[4] = {0x46, 0x4C, 0x41, 0x4E};
const uint8_t CALL_FRAMES_MAX = 124;

typedef struct ErrorInfo {
  uint16_t line;
  const char *line_text;
} ErrorInfo;

typedef struct CallFrame {
  uint8_t ret_addr;
  uint16_t prev_from;
  // TODO: add the rest later
} CallFrame;

typedef struct VM {
  const char *filename;
  Stack stack;
  const uint8_t *inst;

  CallFrame callframes[CALL_FRAMES_MAX];
  size_t callframes_len;

  ErrorInfo *error_info_list;
  size_t error_info_list_len;

  HM globals;  // const char * to Value *
} VM;

typedef enum VMInitResult {
  VM_INIT_SUCCESS,
  VM_INIT_ERR_OPEN_FILE,
  VM_INIT_ERR_READ_FILE,
  VM_INIT_ERR_OUT_OF_MEM,
} VMInitResult;

VMInitResult vm_init(VM *vm, const char *filename);
void vm_deinit(VM *vm);

void print_error(const char *msg);
void print_error_with_stack_trace(const ErrorInfo *err_info, const char *msg);

#endif  // !FVM_H
