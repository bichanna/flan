#ifndef FGC_H
#define FGC_H

#include "stack.h"
#include "value.h"

#define NURSERY_SIZE 2048 * 2048 * 2        // ~8.4 MB
#define NURSING_HOME_SIZE 2048 * 2048 * 16  // ~67.1 MB

// FOLL is short for FObject Linked List
typedef struct FOLLNode {
  FObject *obj;
  struct FOLLNode *next;
} FOLLNode;

typedef struct FOLLNode *FOLinkedList;

typedef struct GC {
  Stack *stack;
  size_t nursery_size;
  size_t nursing_home_size;
  FOLinkedList nursery_list;
  FOLinkedList nursing_home_list;
} GC;

void collect_nursery(GC *gc);
void collect_nursing_home(GC *gc);

void collect_if_needed(GC *gc);

void add_to_nursery(GC *gc, FObject *obj);
void add_to_nursing_home(GC *gc, FObject *obj);
void remove_from_nursery(GC *gc, FObject *obj);
void remove_from_nursing_home(GC *gc, FObject *obj);

GC *init_gc(Stack *stack);
void free_gc(GC *gc);

void register_object(GC *gc, FObject *obj);

#endif  // !FGC_H
