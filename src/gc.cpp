#include "gc.hpp"

#include <string>
#include <typeinfo>
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
      delete obj;  // Clear memory :)
    } else {
      obj->marked = false;
    }
  }

  this->maxObjNum = this->objects.size() * 2;
}

void GC::addObject(Object *object) {
  this->objects.push_back(object);
}

Value GC::createString(std::string value) {
  auto str = new String(value);
  this->addObject(str);
  return str;
}

Value GC::createAtom(std::string value) {
  auto atom = new Atom(value);
  this->addObject(atom);
  return atom;
}

Value GC::createList(std::vector<Value> elements) {
  auto list = new List(elements);
  this->addObject(list);
  return list;
}

Value GC::createTable(std::unordered_map<std::string, Value> hashMap) {
  auto table = new Table(hashMap);
  this->addObject(table);
  return table;
}

Value GC::createTuple(std::vector<Value> values) {
  auto tuple = new Tuple(values);
  this->addObject(tuple);
  return tuple;
}

Value GC::createFunction(std::string name,
                         std::uint16_t arity,
                         std::uint8_t *buffers) {
  auto func = new Function(name, arity, buffers);
  this->addObject(func);
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
      for (std::uint32_t i = 0; i < tuple->values.size(); i++) {
        s += tuple->values.at(i).toString();
        if (i + 1 != tuple->values.size()) s += ", ";
      }
      s += ">";
      return s;
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
      for (std::uint32_t i = 0; i < tuple->values.size(); i++) {
        s += tuple->values.at(i).toDbgString();
        if (i + 1 != tuple->values.size()) s += ", ";
      }
      s += ">";
      return s;
    } else if (typeid(obj) != typeid(String))
      return this->toString();
  }

  return "'" + this->toString() + "'";
}
