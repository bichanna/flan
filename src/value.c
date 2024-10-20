#include "value.h"

#include <stdlib.h>
#include <string.h>

#include "utf8.h"

FValue init_empty_value(void) {
  return (FValue){
      .val_type = VAL_EMPTY,
      .val.i = 0,
  };
}

FValue init_integer_value(long long i) {
  return (FValue){
      .val_type = VAL_INTEGER,
      .val.i = i,
  };
}

FValue init_float_value(double f) {
  return (FValue){
      .val_type = VAL_FLOAT,
      .val.f = f,
  };
}

FValue init_bool_value(bool b) {
  return (FValue){
      .val_type = VAL_BOOL,
      .val.b = b,
  };
}

FValue init_object_value(FObject *obj) {
  return (FValue){
      .val_type = VAL_OBJECT,
      .val.obj = obj,
  };
}

FObject *alloc_string_object(char *str, FObject *prev) {
  FObject *str_obj = malloc(sizeof(FObject));
  str_obj->marked = false;
  str_obj->obj_type = OBJ_STRING;
  str_obj->obj.fstr.str = str;
  str_obj->next = NULL;
  str_obj->free_inner = string_object_free;
  prev->next = str_obj;
  return str_obj;
}

void string_object_free(FObject *str_obj) {
  free(str_obj->obj.fstr.str);
  str_obj->obj.fstr.str = NULL;
}

size_t string_object_utf8_len(FString *fstr) {
  return utf8len(fstr->str);
}

int string_object_concat(FString *dest, FString *src) {
  size_t new_size = strlen(dest->str) + strlen(src->str) - 1;
  char *new_str = realloc(dest->str, new_size);
  if (!new_str) return 1;
  utf8cat(new_str, src->str);
  dest->str = new_str;
  return 0;
}

FObject *alloc_atom_object(const char *str, FObject *prev) {
  FObject *atom_obj = malloc(sizeof(FObject));
  atom_obj->marked = false;
  atom_obj->obj_type = OBJ_ATOM;
  atom_obj->obj.fatom.str = str;
  atom_obj->next = NULL;
  atom_obj->free_inner = atom_object_free;
  prev->next = atom_obj;
  return atom_obj;
}

void atom_object_free(FObject *atom_obj) {
  free((void *)atom_obj->obj.fatom.str);
  atom_obj->obj.fatom.str = NULL;
}

size_t atom_object_utf8_len(FAtom *fatom) {
  return utf8len(fatom->str);
}

FObject *alloc_list_object_with_cap(size_t cap, FObject *prev) {
  FObject *list_obj = malloc(sizeof(FObject));
  list_obj->marked = false;
  list_obj->obj_type = OBJ_LIST;
  list_obj->obj.flist.arr = (FValue *)malloc(sizeof(FValue) * cap);
  list_obj->obj.flist.len = 0;
  list_obj->obj.flist.cap = cap;
  list_obj->next = NULL;
  list_obj->free_inner = list_object_free;
  prev->next = list_obj;
  return list_obj;
}
FObject *alloc_list_object(FObject *prev) {
  return alloc_list_object_with_cap(LIST_ELEM_INIT_CAP, prev);
}

void list_object_free(FObject *list_obj) {
  free(list_obj->obj.flist.arr);
  list_obj->obj.flist.arr = NULL;
  list_obj->obj.flist.len = 0;
  list_obj->obj.flist.cap = 0;
}

void list_object_grow_cap(FList *flist, int by) {
  flist->cap *= by;
  flist->arr = realloc(flist->arr, sizeof(FValue) * flist->cap);
}

void list_object_append(FList *flist, FValue elem) {
  if (++(flist->len) == flist->cap)
    list_object_grow_cap(flist, LIST_GROW_FACTOR);

  flist->arr[flist->len - 1] = elem;
}

int list_object_remove(FList *flist, size_t idx) {
  if (idx >= flist->len) return 1;

  for (size_t i = idx; i < flist->len - 1; i++)
    flist->arr[i] = flist->arr[i + 1];

  return 0;
}

void list_object_pop(FList *flist) {
  list_object_remove(flist, flist->len - 1);
}
