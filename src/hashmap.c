#include "hashmap.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#include "utf8.h"

void hm_init(HM *hm) {
  hm->entries = calloc(INITIAL_HM_CAP, sizeof(HMEntry));
  hm->len = 0;
  hm->cap = INITIAL_HM_CAP;
}

void hm_deinit(HM *hm) {
  for (size_t i = 0; i < hm->cap; i++) {
    free((void *)hm->entries[i].key);
  }

  free(hm->entries);
}

// https://en.wikipedia.org/wiki/Fowler–Noll–Vo_hash_function
static uint64_t hash_key(const char *key) {
  uint64_t hash = FNV_OFFSET;
  for (const char *c = key; *c; c++) {
    hash ^= (uint64_t)(unsigned char)(*c);
    hash *= FNV_PRIME;
  }

  return hash;
}

bool hm_include(HM *hm, const char *key) {
  uint64_t hash = hash_key(key);
  size_t idx = (size_t)(hash & (uint64_t)(hm->cap - 1));

  while (hm->entries[idx].key != NULL) {
    if (utf8cmp(key, hm->entries[idx].key) == 0) return true;
    idx++;
    if (idx >= hm->cap) idx = 0;
  }

  return false;
}

void *hm_get(HM *hm, const char *key) {
  uint64_t hash = hash_key(key);
  size_t idx = (size_t)(hash & (uint64_t)(hm->cap - 1));

  while (hm->entries[idx].key != NULL) {
    if (utf8cmp(key, hm->entries[idx].key) == 0) return hm->entries[idx].value;
    idx++;
    if (idx >= hm->cap) idx = 0;
  }

  return NULL;
}

static const char *hm_set_entry(
    HMEntry *entries, size_t cap, const char *key, void *value, size_t *plen) {
  uint64_t hash = hash_key(key);
  size_t idx = (size_t)(hash & (uint64_t)(cap - 1));

  while (entries[idx].key != NULL) {
    if (utf8cmp(key, entries[idx].key) == 0) {
      entries[idx].value = value;
      return entries[idx].key;
    }

    idx++;
    if (idx >= cap) idx = 0;
  }

  // didn't find the key, so copy if needed and insert it
  if (plen != NULL) {
    key = utf8dup(key);
    if (key == NULL) return NULL;

    (*plen)++;
  }

  entries[idx].key = key;
  entries[idx].value = value;

  return key;
}

static bool hm_grow(HM *hm) {
  size_t new_cap = hm->cap * 2;
  if (new_cap < hm->cap) return false;

  HMEntry *new_entries = calloc(new_cap, sizeof(HMEntry));
  if (new_entries == NULL) return false;

  // move all non-empty ones to new table's entries
  for (size_t i = 0; i < hm->cap; i++) {
    HMEntry entry = hm->entries[i];
    if (entry.key != NULL)
      hm_set_entry(new_entries, new_cap, entry.key, entry.value, NULL);
  }

  free(hm->entries);
  hm->entries = new_entries;
  hm->cap = new_cap;

  return true;
}

const char *hm_set(HM *hm, const char *key, void *value) {
  if (hm->len >= hm->cap * (2.0 / 3))
    if (!hm_grow(hm)) return NULL;

  return hm_set_entry(hm->entries, hm->cap, key, value, &hm->len);
}

void *hm_pop(HM *hm, const char *key) {
  uint64_t hash = hash_key(key);
  size_t idx = (size_t)(hash & (uint64_t)(hm->cap - 1));

  while (hm->entries[idx].key != NULL) {
    if (utf8cmp(key, hm->entries[idx].key) == 0) {
      const char *key = hm->entries[idx].key;
      free((void *)key);
      key = NULL;
      return hm->entries[idx].value;
    }

    idx++;
    if (idx >= hm->cap) idx = 0;
  }

  return NULL;
}

HMIter hm_iter_create(HM *hm) {
  HMIter hmi;
  hmi.hm = hm;
  hmi.idx = 0;
  return hmi;
}

bool hm_iter_next(HMIter *it) {
  HM *hm = it->hm;

  while (it->idx < hm->cap) {
    size_t i = it->idx;
    it->idx++;
    if (hm->entries[i].key != NULL) {
      HMEntry entry = hm->entries[i];
      it->key = entry.key;
      it->value = entry.value;
      return true;
    }
  }

  return false;
}
