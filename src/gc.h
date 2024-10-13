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

GC init_gc(Stack *stack);
void free_gc(GC *gc);

FObject *create_and_register_string_object(GC *gc, char *str);
FObject *create_and_register_atom_object(GC *gc, const char *str);
FObject *create_and_register_list_object_with_cap(GC *gc, size_t cap);
FObject *create_and_register_list_object(GC *gc);

void collect_if_needed(GC *gc);
void collect_nursery(GC *gc);
void collect_nursing_home(GC *gc);

#endif  // !FGC_H
