#pragma once
#include <cstdint>
#include <list>
#include <string>
#include <variant>
#include <vector>

#include "utf8.hpp"

namespace flan {

struct Object;

struct Value {
  std::variant<char, std::int64_t, double, bool, Object*> value;

  Value() : value{static_cast<char>(0)} {};
  Value(std::int64_t value) : value{value} {};
  Value(double value) : value{value} {};
  Value(bool value) : value{value} {};
  Value(Object* obj) : value{obj} {};

  std::string toString();
  std::string toDbgString();
  bool truthy();
};

struct Object {
  bool marked{false};
  void mark();
};

struct String : public Object {
  std::string value;
  std::size_t length;
  String(std::string value) : value{value}, length{utf8len(value.c_str())} {};
};

struct Atom : public Object {
  std::string value;
  std::size_t length;
  Atom(std::string value) : value{value}, length{utf8len(value.c_str())} {};
};

enum class EitherFlag : char { Left, Right };

struct Either : public Object {
  Value value;
  EitherFlag flag;
  Either(Value value, EitherFlag flag) : value{value}, flag{flag} {};
  bool isLeft();
};

class GC {
 private:
  std::size_t maxObjNum = 126;
  std::list<Object*> objects;
  std::vector<Value>* stack;

  void perform();
  void mayPerform();

 public:
  GC(std::vector<Value>* stack) : stack{stack} {};
  void addObject(Object* object);
};
}  // namespace flan
