#ifndef FSTACK_H
#define FSTACK_H

#include <stddef.h>
#include <stdint.h>

#include "value.h"

#define INITIAL_STACK_SIZE 256

typedef struct Stack {
  FValue *arr;
  size_t len;
  size_t cap;
  uint8_t from;
} Stack;

Stack init_stack();
void free_stack(Stack *stack);

void stack_push(Stack *stack, FValue value);
FValue stack_pop(Stack *stack);
FValue stack_last_value(Stack *stack);
FValue stack_at(Stack *stack, size_t index);
FValue stack_from_end(Stack *stack, size_t end_index);
void stack_set_from(Stack *stack, uint8_t new_from);

#endif  // !FSTACK_H
