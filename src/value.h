#ifndef FVALUE_H
#define FVALUE_H

#include <float.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define LIST_ELEM_INIT_CAP 2

typedef enum ObjectType {
  OBJ_STRING,
  OBJ_ATOM,
  OBJ_LIST,
} ObjectType;

typedef struct FObject {
  bool marked;
  ObjectType obj_type;
  size_t (*size)();
  void (*free_inner)(void *);
} FObject;

typedef enum ValueType {
  VAL_EMPTY,
  VAL_INTEGER,
  VAL_FLOAT,
  VAL_BOOL,
  VAL_OBJECT,
} ValueType;

typedef struct FValue {
  ValueType val_type;
  union {
    int64_t i;
    double f;
    bool b;
    FObject *obj;
  };
} FValue;

typedef struct FString {
  FObject obj;
  char *str;
} FString;

typedef struct FAtom {
  FObject obj;
  const char *str;
} FAtom;

typedef struct FList {
  FObject obj;
  FObject **elems;
  int64_t len;
  int64_t cap;
} FList;

FValue init_empty_value();
FValue init_integer_value(long long i);
FValue init_float_value(double f);
FValue init_bool_value(bool b);
FValue init_object_value(FObject *obj);

FObject init_object(ObjectType obj_type,
                    size_t (*size)(),
                    void (*free_inner)(void *));

size_t string_object_size();
void string_object_free_inner(void *string_obj);
FString *init_string_object(char *str);

size_t atom_object_size();
void atom_object_free_inner(void *atom_obj);
FAtom *init_atom_object(const char *str);

size_t list_object_size();
void list_object_free_inner(void *list_obj);
FList *init_list_object_with_cap(int64_t cap);
FList *init_list_object();

#endif  // !FVALUE_H
