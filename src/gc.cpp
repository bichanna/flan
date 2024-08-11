#include "gc.hpp"

#include <string>
#include <typeinfo>
#include <variant>

#include "utf8.hpp"

using namespace flan;

void Object::mark() {
  if (this->marked) return;
  this->marked = true;
}

void GC::mayGC() {
  mayGCNursery();
  mayGCRetirementHome();
}

void GC::mayGCNursery() {
  if (this->nurseryHeap >= this->maxNurserySize) this->gcNursery();
}

void GC::mayGCRetirementHome() {
  if (this->retirementHomeHeap >= this->maxRetirementHomeSize)
    this->gcRetirementHome();
}

void GC::gcNursery() {
  // Mark all
  for (auto value : *this->stack) {
    if (std::holds_alternative<Object *>(value.value))
      std::get<Object *>(value.value)->mark();
  }

  // Sweep
  for (auto it = this->nursery.begin(); it != this->nursery.end(); it++) {
    auto obj = *it;
    this->nursery.erase_after(it);
    this->nurseryHeap -= obj->byteSize();

    if (!obj->marked) {
      delete obj;  // Clear memory :)
    } else {
      obj->marked = false;
      this->retirementHome.push_front(obj);
    }
  }
}

void GC::gcRetirementHome() {
  // Mark all
  for (auto value : *this->stack) {
    if (std::holds_alternative<Object *>(value.value))
      std::get<Object *>(value.value)->mark();
  }

  // Sweep
  for (auto obj : this->nursery) {
    if (!obj->marked) {
      this->retirementHome.remove(obj);
      this->retirementHomeHeap -= obj->byteSize();
      delete obj;  // Clear memory :)
    } else {
      obj->marked = false;
    }
  }
}

std::size_t String::utf8length() {
  return utf8len(this->value.c_str());
}

std::size_t Atom::utf8length() {
  return utf8len(this->value);
}

void GC::addObject(Object *object) {
  this->mayGC();
  this->nursery.push_front(object);
}

Value GC::createString(std::string value) {
  auto str = new String(value);
  this->addObject(str);
  this->nurseryHeap += sizeof(String);
  return str;
}

Value GC::createAtom(const char *value, const std::size_t byte_length) {
  auto atom = new Atom(value, byte_length);
  this->addObject(atom);
  this->nurseryHeap += sizeof(Atom);
  return atom;
}

Value GC::createList(std::vector<Value> elements) {
  auto list = new List(elements);
  this->addObject(list);
  this->nurseryHeap += sizeof(List);
  return list;
}

Value GC::createTable(std::unordered_map<std::string, Value> hashMap) {
  auto table = new Table(hashMap);
  this->addObject(table);
  this->nurseryHeap += sizeof(Table);
  return table;
}

Value GC::createTuple(Value *values, std::uint8_t length) {
  auto tuple = new Tuple(values, length);
  this->addObject(tuple);
  this->nurseryHeap += sizeof(Tuple);
  return tuple;
}

Value GC::createRawFunction(std::string name,
                            std::uint16_t arity,
                            std::uint8_t *buffers) {
  auto func = new RawFunction(name, arity, buffers);
  this->addObject(func);
  this->nurseryHeap += sizeof(RawFunction);
  return func;
}

bool Value::truthy() {
  if (std::holds_alternative<std::int64_t>(this->value)) {
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

std::string Value::toString() {
  if (std::holds_alternative<char>(this->value)) {
    return "_";
  } else if (std::holds_alternative<std::int64_t>(this->value)) {
    return std::to_string(std::get<std::int64_t>(this->value));
  } else if (std::holds_alternative<double>(this->value)) {
    return std::to_string(std::get<double>(this->value));
  } else if (std::holds_alternative<bool>(this->value)) {
    return std::to_string(std::get<bool>(this->value));
  } else if (std::holds_alternative<Object *>(this->value)) {
    auto obj = std::get<Object *>(this->value);
    if (typeid(obj) == typeid(String))
      return static_cast<String *>(obj)->value;
    else if (typeid(obj) == typeid(Atom))
      return static_cast<Atom *>(obj)->value;
    else if (typeid(obj) == typeid(List)) {
      auto list = static_cast<List *>(obj);
      std::string s{"["};
      for (std::uint32_t i = 0; i < list->elements.size(); i++) {
        s += list->elements.at(i).toString();
        if (i + 1 != list->elements.size()) s += ", ";
      }
      s += "]";
      return s;
    } else if (typeid(obj) == typeid(Table)) {
      auto table = static_cast<Table *>(obj);
      std::string s{"{"};
      std::size_t count = 0;
      for (auto &pair : table->hashMap) {
        count++;
        s += pair.first + ": " + pair.second.toString();
        if (count + 1 != table->hashMap.size()) s += ", ";
      }
      s += "}";
      return s;
    } else if (typeid(obj) == typeid(Tuple)) {
      auto tuple = static_cast<Tuple *>(obj);
      std::string s{"<"};
      for (std::uint32_t i = 0; i < tuple->length; i++) {
        s += tuple->values[i].toString();
        if (i + 1 != tuple->length) s += ", ";
      }
      s += ">";
      return s;
    } else if (typeid(obj) == typeid(RawFunction)) {
      auto func = static_cast<RawFunction *>(obj);
      return "<function " + func->name + ">";
    }
  }

  return "::UNKNOWN VALUE::";
}

std::string Value::toDbgString() {
  if (!std::holds_alternative<Object *>(this->value)) {
    auto obj = std::get<Object *>(this->value);
    if (typeid(obj) == typeid(List)) {
      auto list = static_cast<List *>(obj);
      std::string s{"["};
      for (std::uint32_t i = 0; i < list->elements.size(); i++) {
        s += list->elements.at(i).toDbgString();
        if (i + 1 != list->elements.size()) s += ", ";
      }
      s += "]";
      return s;
    } else if (typeid(obj) == typeid(Table)) {
      auto table = static_cast<Table *>(obj);
      std::string s{"{"};
      std::size_t count = 0;
      for (auto &pair : table->hashMap) {
        count++;
        s += pair.first + ": " + pair.second.toDbgString();
        if (count + 1 != table->hashMap.size()) s += ", ";
      }
      s += "}";
      return s;
    } else if (typeid(obj) == typeid(Tuple)) {
      auto tuple = static_cast<Tuple *>(obj);
      std::string s{"<"};
      for (std::uint32_t i = 0; i < tuple->length; i++) {
        s += tuple->values[i].toDbgString();
        if (i + 1 != tuple->length) s += ", ";
      }
      s += ">";
      return s;
    } else if (typeid(obj) != typeid(String))
      return this->toString();
  }

  return "'" + this->toString() + "'";
}
