#pragma once
#include <cstdint>
#include <list>
#include <string>
#include <unordered_map>
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
  std::size_t utf8length;
  String(std::string value)
      : value{value}, utf8length{utf8len(value.c_str())} {};
};

struct Atom : public Object {
  std::string value;
  std::size_t utf8length;
  Atom(std::string value) : value{value}, utf8length{utf8len(value.c_str())} {};
};

enum class EitherFlag : char { Left, Right };

struct Either : public Object {
  Value value;
  EitherFlag flag;
  Either(Value value, EitherFlag flag) : value{value}, flag{flag} {};
  bool isLeft();
};

struct List : public Object {
  std::vector<Value> elements;
  List(std::vector<Value> elements) : elements{elements} {};
};

struct Table : public Object {
  std::unordered_map<std::string, Value> hashMap;
  Table(std::unordered_map<std::string, Value> hashMap) : hashMap{hashMap} {};
};

struct Tuple : public Object {
  std::vector<Value> values;
  Tuple(std::vector<Value> values) : values{values} {};
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
