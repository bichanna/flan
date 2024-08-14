#include "value.hpp"

#include <sstream>
#include <typeinfo>

#include "utf8.hpp"

using namespace flan;

void Object::mark() {
  if (this->marked) return;
  this->marked = true;
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

void Upvalue::mark() {
  if (this->marked) return;
  this->marked = true;

  if (std::holds_alternative<Object *>(this->value.value))
    std::get<Object *>(this->value.value)->mark();
}

void Closure::mark() {
  if (this->marked) return;
  this->marked = true;

  this->function->mark();
  for (auto i = 0; i < this->upvalueCount; i++) this->upvalues[i]->mark();
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
    } else if (typeid(obj) == typeid(Function)) {
      auto func = static_cast<Function *>(obj);
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

std::size_t String::utf8length() {
  return utf8len(this->value.c_str());
}

std::size_t Atom::utf8length() {
  return utf8len(this->value);
}
