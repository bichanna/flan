#ifndef FGC_H
#define FGC_H

#include "stack.h"
#include "value.h"

#define NURSERY_SIZE 2048 * 2048 * 2        // ~8.4 MB
#define NURSING_HOME_SIZE 2048 * 2048 * 16  // ~67.1 MB

typedef struct GC {
  Stack *stack;
  size_t nursery_size;
  size_t nursing_home_size;
  FObject *nursery_list;
  FObject *nursing_home_list;
} GC;

GC gc_init(Stack *stack);
void gc_deinit(GC *gc);

FObject *string_object_create_and_register(GC *gc, char *str);
FObject *atom_object_create_and_register(GC *gc, const char *str);
FObject *list_object_create_and_register_with_cap(GC *gc, size_t cap);
FObject *list_object_create_and_register(GC *gc);

void gc_collect_if_needed(GC *gc);
void gc_collect_nursery(GC *gc);
void gc_collect_nursing_home(GC *gc);

#endif  // !FGC_H
