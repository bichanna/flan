#include "gc.hpp"

#include <string>
#include <variant>

using namespace flan;

GC::GC(std::vector<Value> *stack) : stack{stack} {
  this->atomHeap = static_cast<Atom *>(std::malloc(maxAtomHeapSize));
}

GC::~GC() {
  std::free(this->atomHeap);
}

void GC::GCIfNeeded() {
  if (this->nurseryHeap >= this->maxNurserySize) {
    if (this->retirementHomeHeap >= this->maxRetirementHomeSize)
      this->GCRetirementHome();

    this->GCNursery();
  }
}

void GC::GCNursery() {
  // Mark all
  for (auto value : *this->stack)
    if (std::holds_alternative<Object *>(value.value))
      std::get<Object *>(value.value)->mark();

  // Sweep
  for (auto obj : this->nursery) {
    this->removeFromNursery(obj);
    if (!obj->marked) {
      delete obj;  // Clear memory :)
    } else {
      obj->marked = false;
      this->addToRetirementHome(obj);
    }
  }
}

void GC::GCRetirementHome() {
  // No need for marking
  // Sweep
  for (auto obj : this->retirementHome) {
    if (!obj->marked) {
      this->removeFromRetirementHome(obj);
      delete obj;  // Clear memory :)
    } else {
      obj->marked = false;
    }
  }
}

void GC::addToNursery(Object *obj) {
  this->GCIfNeeded();
  this->nursery.push_front(obj);
  this->nurseryHeap += obj->byteSize();
}

void GC::addToRetirementHome(Object *obj) {
  this->retirementHome.push_front(obj);
  this->retirementHomeHeap += obj->byteSize();
}

void GC::removeFromNursery(Object *obj) {
  this->nursery.remove(obj);
  this->nurseryHeap -= obj->byteSize();
}

void GC::removeFromRetirementHome(Object *obj) {
  this->retirementHome.remove(obj);
  this->retirementHomeHeap -= obj->byteSize();
}

Value GC::createString(std::string value) {
  auto str = new String(value);
  this->addToNursery(str);
  return str;
}

Value GC::createAtom(char *value, std::size_t byte_length) {
  auto atom = &atomHeap[atomHeapCount++];
  atom->value = value;
  atom->byte_length = byte_length;
  return atom;
}

Value GC::createList(std::vector<Value> elements) {
  auto list = new List(elements);
  this->addToNursery(list);
  return list;
}

Value GC::createTable(std::unordered_map<std::string, Value> hashMap) {
  auto table = new Table(hashMap);
  this->addToNursery(table);
  return table;
}

Value GC::createTuple(Value *values, std::uint8_t length) {
  auto tuple = new Tuple(values, length);
  this->addToNursery(tuple);
  return tuple;
}

Value GC::createFunction(const char *name,
                         std::uint16_t arity,
                         std::uint8_t *buffers) {
  auto func = new Function(name, arity, buffers);
  this->addToNursery(func);
  return func;
}

Value GC::createUpvalue(Value value) {
  auto upvalue = new Upvalue(value);
  this->addToNursery(upvalue);
  return upvalue;
}

Upvalue *GC::createUpvaluePtr(Value value) {
  auto upvalue = new Upvalue(value);
  this->addToNursery(upvalue);
  return upvalue;
}

Value GC::createClosure(Function *Function,
                        Upvalue **upvalues,
                        std::uint8_t upvalueCount) {
  auto clos = new Closure(Function, upvalues, upvalueCount);
  this->addToNursery(clos);
  return clos;
}
