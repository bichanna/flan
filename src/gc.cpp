#include "gc.hpp"

#include <variant>

using namespace impala;

void Object::mark() {
  if (this->marked) return;
  this->marked = true;
}

void GC::mayPerform() {
  if (this->objects.size() >= this->maxObjNum) this->perform();
}

void GC::perform() {
  // Mark all
  for (auto value : *this->stack) {
    if (std::holds_alternative<Object *>(value.value))
      std::get<Object *>(value.value)->mark();
  }

  // Sweep
  for (auto obj : this->objects) {
    if (!obj->marked) {
      this->objects.remove(obj);
      delete obj;
    } else {
      obj->marked = false;
    }
  }

  this->maxObjNum = this->objects.size() * 2;
}

void GC::addObject(Object *object) {
  this->objects.push_back(object);
}
