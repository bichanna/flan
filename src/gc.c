#include "gc.h"

#include <stdlib.h>

#include "value.h"

GC *create_gc(Stack *stack) {
  GC *gc = malloc(sizeof(GC));
  gc->stack = stack;
  gc->nursery_size = 0;
  gc->nursing_home_size = 0;
  gc->nursery_list = NULL;
  gc->nursing_home_list = NULL;
  return gc;
}

void free_gc(GC *gc) {
  // forcefully free every single node and its object
  FObject *current_node = gc->nursing_home_list;
  while (current_node != NULL) {
    current_node->free_inner(current_node);
    free(current_node);
  }

  // then free the garbage collector
  free(gc);
}

FObject *create_and_register_string_object(GC *gc, char *str) {
  collect_if_needed(gc);
  gc->nursery_size += sizeof(FObject);
  return create_string_object(str, gc->nursery_list);
}

FObject *create_and_register_atom_object(GC *gc, const char *str) {
  collect_if_needed(gc);
  gc->nursery_size += sizeof(FObject);
  return create_atom_object(str, gc->nursery_list);
}

FObject *create_and_register_list_object_with_cap(GC *gc, size_t cap) {
  collect_if_needed(gc);
  gc->nursery_size += sizeof(FObject);
  return create_list_object_with_cap(cap, gc->nursery_list);
}

FObject *create_and_register_list_object(GC *gc) {
  collect_if_needed(gc);
  gc->nursery_size += sizeof(FObject);
  return create_list_object(gc->nursery_list);
}

void collect_if_needed(GC *gc) {
  if (gc->nursery_size >= NURSERY_SIZE) {
    // mark all
    for (size_t i = 0; i < gc->stack->len; i++)
      if (gc->stack->arr[i].val_type == VAL_OBJECT)
        gc->stack->arr[i].val.obj->marked = true;

    collect_nursery(gc);

    if (gc->nursing_home_size >= NURSING_HOME_SIZE) {
      collect_nursing_home(gc);
    }
  }
}

void collect_nursery(GC *gc) {
  FObject *current_node = gc->nursery_list;
  while (current_node != NULL) {
    gc->nursery_list = current_node->next;

    if (!current_node->marked) {
      gc->nursery_size -= sizeof(FObject);

      FObject *to_be_freed = current_node;
      current_node = to_be_freed->next;
      // free memory
      to_be_freed->free_inner(to_be_freed);
      free(to_be_freed);
    } else {
      // if not collected, move it to nursing home bc it's old
      current_node->marked = false;
      FObject *next_node = current_node->next;

      // move the object to nursing home
      current_node->next = gc->nursing_home_list;
      gc->nursing_home_list = current_node;

      current_node = next_node;
    }
  }
}

void collect_nursing_home(GC *gc) {
  FObject *current_node = gc->nursing_home_list;
  while (current_node != NULL) {
    if (!current_node->marked) {
      // remove from nursing home
      gc->nursing_home_size -= sizeof(FObject);

      FObject *to_be_freed = current_node;
      current_node = to_be_freed->next;
      // free memory
      to_be_freed->free_inner(to_be_freed);
      free(to_be_freed);
    }
  }
}
