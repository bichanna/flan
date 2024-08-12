#include "gc.hpp"

#include <sstream>
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
  if (this->nurseryHeap >= this->maxNurserySize) this->GCNursery();
}

void GC::mayGCRetirementHome() {
  if (this->retirementHomeHeap >= this->maxRetirementHomeSize)
    this->GCRetirementHome();
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
  // Mark all
  for (auto value : *this->stack)
    if (std::holds_alternative<Object *>(value.value))
      std::get<Object *>(value.value)->mark();

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

std::size_t String::utf8length() {
  return utf8len(this->value.c_str());
}

std::size_t Atom::utf8length() {
  return utf8len(this->value);
}

Value GC::createString(std::string value) {
  auto str = new String(value);
  this->addToNursery(str);
  this->nurseryHeap += sizeof(String);
  return str;
}

Value GC::createAtom(const char *value, const std::size_t byte_length) {
  auto atom = new Atom(value, byte_length);
  this->addToNursery(atom);
  this->nurseryHeap += sizeof(Atom);
  return atom;
}

Value GC::createList(std::vector<Value> elements) {
  auto list = new List(elements);
  this->addToNursery(list);
  this->nurseryHeap += sizeof(List);
  return list;
}

Value GC::createTable(std::unordered_map<std::string, Value> hashMap) {
  auto table = new Table(hashMap);
  this->addToNursery(table);
  this->nurseryHeap += sizeof(Table);
  return table;
}

Value GC::createTuple(Value *values, std::uint8_t length) {
  auto tuple = new Tuple(values, length);
  this->addToNursery(tuple);
  this->nurseryHeap += sizeof(Tuple);
  return tuple;
}

Value GC::createRawFunction(const char *name,
                            std::uint16_t arity,
                            std::uint8_t *buffers) {
  auto func = new RawFunction(name, arity, buffers);
  this->addToNursery(func);
  this->nurseryHeap += sizeof(RawFunction);
  return func;
}

void List::mark() {
  if (this->marked) return;
  this->marked = true;

  for (auto &val : this->elements)
    if (std::holds_alternative<Object *>(val.value))
      std::get<Object *>(val.value)->mark();
}

void Table::mark() {
  if (this->marked) return;
  this->marked = true;

  for (auto &p : this->hashMap)
    if (std::holds_alternative<Object *>(p.second.value))
      std::get<Object *>(p.second.value)->mark();
}

void Tuple::mark() {
  if (this->marked) return;
  this->marked = true;

  for (auto i = 0; i < length; i++)
    if (std::holds_alternative<Object *>(this->values[i].value))
      std::get<Object *>(this->values[i].value)->mark();
}

void Closure::mark() {
  if (this->marked) return;
  this->marked = true;
  this->function->mark();
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
      std::stringstream res;
      res << "<function";

      if (func->name)
        res << " " << func->name;
      else
        res << "@" << std::hex << static_cast<void *>(func);

      res << ">";
      return res.str();
    } else if (typeid(obj) == typeid(Closure)) {
      auto func = static_cast<Closure *>(obj)->function;
      std::stringstream res;
      res << "<function";

      if (func->name)
        res << " " << func->name;
      else
        res << "@" << std::hex << static_cast<void *>(func);

      res << ">";
      return res.str();
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
