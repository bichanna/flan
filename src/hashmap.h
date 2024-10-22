#ifndef FHM_H
#define FHM_H

#include "stdbool.h"
#include "stddef.h"

#define INITIAL_HM_CAP 8

#define FNV_OFFSET 14695981039346656037UL
#define FNV_PRIME 1099511628211UL

typedef struct HMEntry {
  const char *key;  // this is NULL when this slot is empty
  void *value;
} HMEntry;

typedef struct HM {
  HMEntry *entries;
  size_t len;
  size_t cap;
} HM;

void hm_init(HM *hm);
void hm_deinit(HM *hm);

bool hm_include(HM *hm, const char *key);
void *hm_get(HM *hm, const char *key);
const char *hm_set(HM *hm, const char *key, void *value);
void *hm_pop(HM *hm, const char *key);

#endif  // !FHM_H
