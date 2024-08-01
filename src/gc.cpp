#include "gc.hpp"

#include <variant>

using namespace flan;

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

bool Value::truthy() {
  if (std::holds_alternative<char>(this->value)) {
    auto v = std::get<char>(this->value);
    return v == 0;
  } else if (std::holds_alternative<std::int64_t>(this->value)) {
    auto v = std::get<std::int64_t>(this->value);
    return v != 0;
  } else if (std::holds_alternative<double>(this->value)) {
    auto v = std::get<double>(this->value);
    return v != 0.0;
  } else if (std::holds_alternative<bool>(this->value)) {
    auto v = std::get<bool>(this->value);
    return v;
  } else {
    return true;
  }
}
