#include "vm.h"

#include <stdio.h>
#include <stdlib.h>

#include "stack.h"

VMInitResult vm_init(VM *vm, const char *filename) {
  FILE *input_file = fopen(filename, "r");
  if (input_file == NULL) return VM_INIT_ERR_OPEN_FILE;

  fseek(input_file, 0, SEEK_END);
  size_t buflen = ftell(input_file);
  fseek(input_file, 0, SEEK_SET);

  vm->inst = malloc(buflen + 1);
  if (vm->inst == NULL) return VM_INIT_ERR_OUT_OF_MEM;

  if (fread((uint8_t *)vm->inst, 1, buflen, input_file) != buflen) {
    fclose(input_file);
    free((void *)vm->inst);
    return VM_INIT_ERR_READ_FILE;
  }

  uint8_t *buf = (uint8_t *)vm->inst;
  buf[buflen] = '\0';
  fclose(input_file);

  return VM_INIT_SUCCESS;
}

void vm_deinit(VM *vm) {
  stack_deinit(&vm->stack);
  free((void *)vm->inst);
  free(vm->error_info_list);

  HMEntry *entries = vm->globals.entries;
  for (size_t i = 0; i < vm->globals.cap; i++)
    if (entries[i].value != NULL) free(entries[i].value);
}

void print_error(const char *msg) {
  printf("Error: %s\n", msg);
}

void print_error_with_stack_trace(const ErrorInfo *err_info, const char *msg) {
  // TODO: implement this later
}

static uint8_t read_uint8(const uint8_t *ptr) {
  uint8_t value = *ptr;
  ptr++;
  return value;
}

static uint16_t read_uint16(const uint8_t *ptr) {
  uint8_t b1 = read_uint8(ptr);
  uint8_t b0 = read_uint8(ptr);
  return (uint16_t)(b0) << 8 | (uint16_t)(b1) & 0xFF;
}

static uint32_t read_uint32(const uint8_t *ptr) {
  uint8_t b3 = read_uint8(ptr);
  uint8_t b2 = read_uint8(ptr);
  uint8_t b1 = read_uint8(ptr);
  uint8_t b0 = read_uint8(ptr);
  return (uint32_t)(b0) | (uint32_t)(b1) << 8 | (uint32_t)(b2) << 16 |
         (uint32_t)(b3) << 24;
}
