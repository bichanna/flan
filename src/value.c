#include "value.h"

#include <stdlib.h>

FValue init_empty_value() {
  return (FValue){
      .val_type = VAL_EMPTY,
      .obj = NULL,
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
                    size_t (*size)(),
                    void (*free_inner)(void *)) {
  return (FObject){.marked = false,
                   .obj_type = obj_type,
                   .size = size,
                   .free_inner = free_inner};
}

size_t string_object_size() {
  return sizeof(FString);
}

void string_object_free_inner(void *string_obj) {
  FString *str_obj = (FString *)string_obj;
  free(str_obj->str);
  str_obj->str = NULL;
}

FString *init_string_object(char *str) {
  FString *str_obj = (FString *)malloc(string_object_size());
  str_obj->obj =
      init_object(OBJ_STRING, string_object_size, string_object_free_inner);
  str_obj->str = str;
  return str_obj;
}

size_t atom_object_size() {
  return sizeof(FAtom);
}

void atom_object_free_inner(void *atom_obj) {
  FAtom *atom = (FAtom *)atom_obj;
  free((void *)atom->str);
  atom->str = NULL;
}

FAtom *init_atom_object(const char *str) {
  FAtom *atom_obj = (FAtom *)malloc(atom_object_size());
  atom_obj->obj =
      init_object(OBJ_ATOM, atom_object_size, atom_object_free_inner);
  atom_obj->str = str;
  return atom_obj;
}

size_t list_object_size() {
  return sizeof(FList);
}

void list_object_free_inner(void *list_obj) {
  FList *list = (FList *)list_obj;
  free(list->elems);
  list->elems = NULL;
  list->len = 0;
  list->cap = 0;
}

FList *init_list_object_with_cap(int64_t cap) {
  FList *list_obj = (FList *)malloc(list_object_size());
  list_obj->obj =
      init_object(OBJ_LIST, list_object_size, list_object_free_inner);
  list_obj->elems = (FObject **)malloc(sizeof(FObject *) * cap);
  list_obj->len = 0;
  list_obj->cap = cap;
  return list_obj;
}

FList *init_list_object() {
  return init_list_object_with_cap(LIST_ELEM_INIT_CAP);
}
