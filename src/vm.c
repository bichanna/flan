#include "vm.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "utf8.h"
#include "gc.h"
#include "stack.h"
#include "value.h"

VMInitResult vm_init(VM *vm, const char *filename) {
  FILE *input_file = fopen(filename, "r");
  if (input_file == NULL) return VM_INIT_ERR_OPEN_FILE;

  fseek(input_file, 0, SEEK_END);
  size_t buflen = ftell(input_file);
  fseek(input_file, 0, SEEK_SET);

  vm->inst = malloc(buflen + 1);
  if (vm->inst == NULL) return VM_INIT_ERR_OUT_OF_MEM;

  if (fread(vm->inst, 1, buflen, input_file) != buflen) {
    fclose(input_file);
    free(vm->inst);
    return VM_INIT_ERR_READ_FILE;
  }

  vm->inst[buflen] = '\0';
  fclose(input_file);

  gc_init(&vm->gc, &vm->stack);

  return VM_INIT_SUCCESS;
}

void vm_deinit(VM *vm) {
  stack_deinit(&vm->stack);
  gc_deinit(&vm->gc);

  for (size_t i = 0; i < vm->error_info_list_len; i++)
    free((void *)vm->error_info_list[i].line_text);

  free(vm->error_info_list);

  HMEntry *entries = vm->globals.entries;
  for (size_t i = 0; i < vm->globals.cap; i++)
    if (entries[i].value != NULL) free(entries[i].value);

  free(vm->inst);
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
  return (uint16_t)(b0) << 8 | ((uint16_t)(b1) & 0xFF);
}

static uint32_t read_uint32(const uint8_t *ptr) {
  uint8_t b3 = read_uint8(ptr);
  uint8_t b2 = read_uint8(ptr);
  uint8_t b1 = read_uint8(ptr);
  uint8_t b0 = read_uint8(ptr);
  return (uint32_t)(b0) | (uint32_t)(b1) << 8 | (uint32_t)(b2) << 16 |
         (uint32_t)(b3) << 24;
}

static void read_error_info_section(VM *vm) {
  vm->error_info_list_len = read_uint16(vm->inst);
  vm->error_info_list = malloc(sizeof(ErrorInfo) * vm->error_info_list_len);

  for (size_t i = 0; i < vm->error_info_list_len; i++) {
    ErrorInfo err_info;

    err_info.line = read_uint16(vm->inst);

    uint16_t len = read_uint16(vm->inst);
    char *line_text = malloc(len + 1);
    line_text[len] = '\0';
    for (size_t j = 0; j < len; j++) line_text[j] = (char)read_uint8(vm->inst);
    err_info.line_text = line_text;

    vm->error_info_list[i] = err_info;
  }
}

static bool check_magic_number(uint8_t *inst) {
  return (read_uint8(inst) == MAGIC_NUMBER[0]) &&
         (read_uint8(inst) == MAGIC_NUMBER[1]) &&
         (read_uint8(inst) == MAGIC_NUMBER[2]) &&
         (read_uint8(inst) == MAGIC_NUMBER[3]);
}

static bool check_version(uint8_t *inst) {
  return (read_uint8(inst) == VERSION[0]) && (read_uint8(inst) <= VERSION[1]) &&
         (read_uint8(inst) <= VERSION[2]);
}

static void push(VM *vm, FValue value) {
  stack_push(&vm->stack, value);
}

static FValue pop(VM *vm) {
  return stack_pop(&vm->stack);
}

static void jump_forward(uint8_t *inst, size_t offset) {
  inst += offset;
}

static const char *read_short_string(uint8_t *inst) {
  uint8_t len = read_uint8(inst);
  char *str = malloc(len + 1);
  str[len] = '\0';

  for (size_t i = 0; i < len; i++) str[i] = (char)read_uint8(inst);

  return str;
}

static FValue read_integer(uint8_t *inst) {
  uint8_t bytes[4];
  for (size_t i = 0; i < 4; i++) bytes[i] = read_uint8(inst);

  int64_t result = 0;
  for (size_t i = 0; i < 4; i++) result |= (int64_t)(bytes[i]) << (i * 8);

  return create_integer_value(result);
}

static FValue read_float(uint8_t *inst) {
  uint8_t bytes[4];
  for (size_t i = 0; i < 4; i++) bytes[i] = read_uint8(inst);

  double result = 0.0;
  memcpy(&result, bytes, 4);

  return create_float_value(result);
}

static FValue read_string(GC *gc, uint8_t *inst) {
  uint16_t len = read_uint16(inst);
  char *str = malloc(len + 1);
  str[len] = '\0';

  for (size_t i = 0; i < len; i++) str[i] = (char)read_uint8(inst);

  FObject *str_obj = string_object_create_and_register(gc, str);
  return create_object_value(str_obj);
}

static FValue read_atom(GC *gc, uint8_t *inst) {
  uint16_t len = read_uint16(inst);
  char *str = malloc(len + 1);
  str[len] = '\0';

  for (size_t i = 0; i < len; i++) str[i] = (char)read_uint8(inst);

  FObject *atom_obj = atom_object_create_and_register(gc, str);
  return create_object_value(atom_obj);
}

static bool read_value(VM *vm, FValue *value) {
  uint8_t *inst = vm->inst;
  uint8_t type = read_uint8(inst);
  switch (type) {
    case 0:
      *value = read_integer(inst);
      break;
    case 1:
      *value = read_float(inst);
      break;
    case 2:
      *value = create_bool_value(true);
      break;
    case 3:
      *value = create_bool_value(false);
      break;
    case 4:
      *value = create_empty_value();
      break;
    case 5:
      *value = read_string(&vm->gc, inst);
      break;
    case 6:
      *value = read_atom(&vm->gc, inst);
      break;
    default:
      return false;
  }

  return true;
}

static InterpretResult create_interpret_result(InterpretResultType type, const char *err_msg, bool show_stack_trace) {
  InterpretResult res;
  char *msg = malloc(strlen(err_msg) + 1);
  utf8cpy(msg, err_msg);
  res.err_msg = msg;
  res.show_stack_trace = show_stack_trace;
  res.type = type;
  return res;
}

InterpretResult interpret(VM *vm) {
  if (!check_magic_number(vm->inst))
    return create_interpret_result(INTERPRET_ERR, "Invalid Magic number", false);

  if (!check_version(vm->inst))
    return create_interpret_result(INTERPRET_ERR, "Upgrade the Flan runtime", false);

  for (;;) {
    InstructionType inst_type = (InstructionType)read_uint8(vm->inst);

    switch (inst_type) {
      // TODO
    }
  }

  return create_interpret_result(INTERPRET_SUCCESS, NULL, false);
}
