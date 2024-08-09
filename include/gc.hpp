#pragma once
#include <cstdint>
#include <forward_list>
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
  virtual ~Object() {};
  virtual std::uint64_t byteSize() {
    return sizeof(Object);
  };
};

struct String : public Object {
  std::string value;
  std::size_t utf8length;
  String(std::string value)
      : value{value}, utf8length{utf8len(value.c_str())} {};
  ~String() override {};
  std::uint64_t byteSize() override {
    return sizeof(String);
  };
};

struct Atom : public Object {
  std::string value;
  std::size_t utf8length;
  Atom(std::string value) : value{value}, utf8length{utf8len(value.c_str())} {};
  ~Atom() override {};
  std::uint64_t byteSize() override {
    return sizeof(Atom);
  };
};

struct List : public Object {
  std::vector<Value> elements;
  List(std::vector<Value> elements) : elements{elements} {};
  ~List() override {};
  std::uint64_t byteSize() override {
    return sizeof(List);
  };
};

struct Table : public Object {
  std::unordered_map<std::string, Value> hashMap;
  Table(std::unordered_map<std::string, Value> hashMap) : hashMap{hashMap} {};
  ~Table() override {};
  std::uint64_t byteSize() override {
    return sizeof(Table);
  };
};

struct Tuple : public Object {
  std::vector<Value> values;
  Tuple(std::vector<Value> values) : values{values} {};
  ~Tuple() override {};
  std::uint64_t byteSize() override {
    return sizeof(Tuple);
  };
};

struct Function : public Object {
  std::string name;
  std::uint16_t arity;
  std::uint8_t* buffers;
  Function(std::string name, std::uint16_t arity, std::uint8_t* buffers)
      : name{name}, arity{arity}, buffers{buffers} {};
  ~Function() override {
    delete[] this->buffers;
  };
  std::uint64_t byteSize() override {
    return sizeof(Function);
  };
};

class GC {
 private:
  const std::size_t maxNurserySize = 1024 * 256;          // ~262KB
  const std::size_t maxRetirementHomeSize = 1024 * 2048;  // ~2MB
  std::vector<Value>* stack;

  std::size_t retirementHomeHeap = 0;
  std::forward_list<Object*> retirementHome;

  std::size_t nurseryHeap = 0;
  std::forward_list<Object*> nursery;

  void mayGCNursery();
  void gcNursery();

  void mayGCRetirementHome();
  void gcRetirementHome();

  void mayGC();

 public:
  GC(std::vector<Value>* stack) : stack{stack} {};
  void addObject(Object* object);
  Value createString(std::string value);
  Value createAtom(std::string value);
  Value createList(std::vector<Value> elements);
  Value createTable(std::unordered_map<std::string, Value> hashMap);
  Value createTuple(std::vector<Value> values);
  Value createFunction(std::string name,
                       std::uint16_t arity,
                       std::uint8_t* buffers);
};
}  // namespace flan
