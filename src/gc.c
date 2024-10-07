#include "gc.h"

#include <stdlib.h>

#include "value.h"

void register_object(GC *gc, FObject *obj) {
  add_to_nursery(gc, obj);
}

void add_to_nursery(GC *gc, FObject *obj) {
  collect_if_needed(gc);
  gc->nursery_size += obj->size();

  FOLLNode *node = malloc(sizeof(FOLLNode));
  node->obj = obj;
  node->next = gc->nursery_list;
  gc->nursery_list = node;
}

void add_to_nursing_home(GC *gc, FObject *obj) {
  gc->nursing_home_size += obj->size();

  FOLLNode *node = malloc(sizeof(FOLLNode));
  node->obj = obj;
  node->next = gc->nursing_home_list;
  gc->nursing_home_list = node;
}

void collect_nursery(GC *gc) {
  // mark all
  for (size_t i = 0; i < gc->stack->len; i++)
    if (gc->stack->arr[i].val_type == VAL_OBJECT)
      gc->stack->arr[i].obj->marked = true;

  // sweep
  FOLLNode *current_node = gc->nursery_list;
  do {
    FObject *obj = current_node->obj;
    current_node =
        current_node->next;  // point to the next FOLLNode bc it'll be removed
    remove_from_nursery(gc, obj);  // this also frees the FOLLNode
    if (!obj->marked) {
      obj->free(obj);  // free the inner memory
      free(obj);       // actually free the object itself
    } else {
      obj->marked = false;
      // move the object to the nursing home bc it's old enough
      add_to_nursing_home(gc, obj);
    }
  } while (current_node->next != NULL);
}

void collect_nursing_home(GC *gc) {
  // no need for marking

  // sweep
  FOLLNode *current_node = gc->nursing_home_list;
  do {
    FObject *obj = current_node->obj;
    current_node =
        current_node->next;  // point to the next FOLLNode bc it'll be removed
    if (!obj->marked) {
      remove_from_nursing_home(gc, obj);  // this also frees the FOLLNode
      obj->free(obj);                     // free the inner memory
      free(obj);
    } else {
      obj->marked = false;  // this old fella is still living
    }
  } while (current_node->next != NULL);
}

void collect_if_needed(GC *gc) {
  if (gc->nursery_size >= NURSERY_SIZE) {
    if (gc->nursing_home_size >= NURSING_HOME_SIZE) {
      collect_nursing_home(gc);
    }

    collect_nursery(gc);
  }
}

void remove_from_nursery(GC *gc, FObject *obj) {
  FOLLNode *prev_node = NULL;
  FOLLNode *current_node = gc->nursery_list;
  while (current_node != NULL) {
    if (current_node->obj == obj) {
      gc->nursery_size -= obj->size();

      FOLLNode *next_node = current_node->next;
      if (prev_node != NULL) {
        prev_node->next = next_node;
      } else {
        gc->nursery_list = next_node;
      }

      // free the node, not the object inside
      free(current_node);
      // then break out of the loop
      return;
    }

    prev_node = current_node;
    current_node = current_node->next;
  }
}

void remove_from_nursing_home(GC *gc, FObject *obj) {
  FOLLNode *prev_node = NULL;
  FOLLNode *current_node = gc->nursing_home_list;
  while (current_node != NULL) {
    if (current_node->obj == obj) {
      gc->nursing_home_size -= obj->size();

      FOLLNode *next_node = current_node->next;
      if (prev_node != NULL) {
        prev_node->next = next_node;
      } else {
        gc->nursing_home_list = next_node;
      }

      // free the node, not the object inside
      free(current_node);
      // then break out of the loop
      return;
    }

    prev_node = current_node;
    current_node = current_node->next;
  }
}

GC *init_gc(Stack *stack) {
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
  FOLLNode *current_node = gc->nursing_home_list;
  while (current_node != NULL) {
    current_node->obj->free(current_node->obj);
    free(current_node->obj);
    free(current_node);
  }

  // then free the garbage collector
  free(gc);
}
