#pragma once

#include <cstdint>
#include <list>
#include <string>
#include <variant>
#include <vector>

namespace flan {

struct Object {
  bool marked{false};
  void mark();
};

struct String : public Object {
  std::string value;
  String(std::string value) : value{value} {};
};

struct Atom : public Object {
  std::string value;
  Atom(std::string value) : value{value} {};
};

struct Value {
  // Empty -> (char)0
  // None  -> (char)1
  std::variant<char, std::int64_t, double, bool, Object*> value{(char)0};

  Value() : value{(char)0} {};
  Value(std::int64_t value) : value{value} {};
  Value(double value) : value{value} {};
  Value(bool value) : value{value} {};
  Value(Object* obj) : value{obj} {};

  std::string toString();
  std::string toDbgString();
  bool truthy();
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
