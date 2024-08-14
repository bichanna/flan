#pragma once
#include <cstdint>
#include <cstdlib>
#include <forward_list>
#include <string>

#include "value.hpp"

namespace flan {

class GC {
 private:
  Atom* atomHeap;
  std::size_t atomHeapCount = 0;
  const std::size_t maxAtomHeapSize = 1024 * 1024;             //  ~1.0 MB
  const std::size_t maxNurserySize = 2048 * 2048 * 2;          //  ~8.4 MB
  const std::size_t maxRetirementHomeSize = 2048 * 2048 * 16;  // ~67.1 MB
  std::vector<Value>* stack;

  std::size_t retirementHomeHeap = 0;
  std::forward_list<Object*> retirementHome;

  std::size_t nurseryHeap = 0;
  std::forward_list<Object*> nursery;

  void GCNursery();
  void GCRetirementHome();

  void GCIfNeeded();

  void addToNursery(Object* obj);
  void addToRetirementHome(Object* obj);
  void removeFromNursery(Object* obj);
  void removeFromRetirementHome(Object* obj);

 public:
  GC(std::vector<Value>* stack);
  ~GC();
  Value createString(std::string value);
  Value createAtom(char* value, std::size_t byte_length);
  Value createList(std::vector<Value> elements);
  Value createTable(std::unordered_map<std::string, Value> hashMap);
  Value createTuple(Value* values, std::uint8_t length);
  Value createFunction(const char* name,
                       std::uint16_t arity,
                       std::uint8_t* buffers);
  Value createUpvalue(Value vlaue);
  Upvalue* createUpvaluePtr(Value value);
  Value createClosure(Function* Function,
                      Upvalue** upvalues,
                      std::uint8_t upvalueCount);
};
}  // namespace flan
