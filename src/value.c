#include "value.h"

#include <stdlib.h>
#include <string.h>

#include "utf8.h"

FValue init_empty_value(void) {
  return (FValue){
      .val_type = VAL_EMPTY,
      .i = 0,
  };
}

FValue init_integer_value(long long i) {
  return (FValue){
      .val_type = VAL_INTEGER,
      .i = i,
  };
}

FValue init_float_value(double f) {
  return (FValue){
      .val_type = VAL_FLOAT,
      .f = f,
  };
}

FValue init_bool_value(bool b) {
  return (FValue){
      .val_type = VAL_BOOL,
      .b = b,
  };
}

FValue init_object_value(FObject *obj) {
  return (FValue){
      .val_type = VAL_OBJECT,
      .obj = obj,
  };
}

FObject init_object(ObjectType obj_type,
                    size_t (*size)(void),
                    void (*free)(void *)) {
  return (FObject){
      .marked = false, .obj_type = obj_type, .size = size, .free = free};
}

size_t string_object_size(void) {
  return sizeof(FString);
}

void string_object_free(void *string_obj) {
  FString *str_obj = (FString *)string_obj;
  free(str_obj->str);
  str_obj->str = NULL;
}

FString *init_string_object(char *str) {
  FString *str_obj = malloc(string_object_size());
  str_obj->obj =
      init_object(OBJ_STRING, string_object_size, string_object_free);
  str_obj->str = str;
  return str_obj;
}

size_t string_object_utf8_len(FString *str_obj) {
  return utf8len(str_obj->str);
}

int string_object_concat(FString *dest, FString *src) {
  char *new_str = realloc(dest->str, strlen(dest->str) + strlen(src->str) - 1);
  if (!new_str) return 1;
  utf8cat(new_str, src->str);
  dest->str = new_str;
  return 0;
}

size_t atom_object_size(void) {
  return sizeof(FAtom);
}

void atom_object_free(void *atom_obj) {
  FAtom *atom = (FAtom *)atom_obj;
  free((void *)atom->str);
  atom->str = NULL;
}

FAtom *init_atom_object(const char *str) {
  FAtom *atom_obj = malloc(atom_object_size());
  atom_obj->obj = init_object(OBJ_ATOM, atom_object_size, atom_object_free);
  atom_obj->str = str;
  return atom_obj;
}

size_t atom_object_utf8_len(FAtom *atom_obj) {
  return utf8len(atom_obj->str);
}

size_t list_object_size(void) {
  return sizeof(FList);
}

void list_object_free(void *list_obj) {
  FList *list = (FList *)list_obj;
  free(list->arr);
  list->arr = NULL;
  list->len = 0;
  list->cap = 0;
}

FList *init_list_object_with_cap(size_t cap) {
  FList *list_obj = malloc(list_object_size());
  list_obj->obj = init_object(OBJ_LIST, list_object_size, list_object_free);
  list_obj->arr = (FObject **)malloc(sizeof(FObject *) * cap);
  list_obj->len = 0;
  list_obj->cap = cap;
  return list_obj;
}

FList *init_list_object() {
  return init_list_object_with_cap(LIST_ELEM_INIT_CAP);
}

void list_object_grow_cap(FList *list_obj, int by) {
  list_obj->cap *= by;
  list_obj->arr = realloc(list_obj->arr, list_obj->cap * sizeof(FObject *));
}

void list_object_append_element(FList *list_obj, FObject *new_elem) {
  if (++(list_obj->len) == list_obj->cap)
    list_object_grow_cap(list_obj, LIST_GROW_FACTOR);

  list_obj->arr[list_obj->len - 1] = new_elem;
}

int list_object_remove(FList *list_obj, size_t index) {
  if (index >= list_obj->len) return 1;

  for (size_t i = index; i < list_obj->len - 1; i++)
    list_obj->arr[i] = list_obj->arr[i + 1];

  list_obj->len--;

  return 0;
}

void list_object_pop(FList *list_obj) {
  list_object_remove(list_obj, list_obj->len - 1);
}
