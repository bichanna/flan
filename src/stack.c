#include "stack.h"

#include <stdlib.h>
#include <string.h>

Stack *init_stack() {
  Stack *stack = malloc(sizeof(Stack));
  stack->arr = malloc(sizeof(FValue) * INITIAL_STACK_SIZE);
  stack->len = 0;
  stack->cap = INITIAL_STACK_SIZE;
  stack->from = 0;
  return stack;
}

void free_stack(Stack *stack) {
  free(stack->arr);
  stack->arr = NULL;
  free(stack);
}

void stack_push(Stack *stack, FValue value) {
  if (++(stack->len) == stack->cap) {
    stack->cap *= 1.5;
    stack->arr = realloc(stack->arr, stack->cap * sizeof(FValue));
  }

  stack->arr[stack->len - 1] = value;
}

FValue stack_pop(Stack *stack) {
  stack->len--;
  return stack->arr[stack->len];
}

FValue stack_last_value(Stack *stack) {
  return stack->arr[stack->len - 1];
}

FValue stack_at(Stack *stack, size_t index) {
  return stack->arr[stack->from + index];
}

FValue stack_from_end(Stack *stack, size_t end_index) {
  return stack->arr[stack->len - 1 - end_index];
}

void stack_set_from(Stack *stack, uint8_t new_from) {
  stack->from = new_from;
}
