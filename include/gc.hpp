#pragma once
#include <cstdint>
#include <forward_list>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

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
  // TODO: Make this use less memory with SSO stuff?
  // TODO: Use std::uint32_t instead of std::size_t?
  std::string value;
  String(std::string value) : value{value} {};
  ~String() override {};
  std::size_t utf8length();
  std::uint64_t byteSize() override {
    return sizeof(String);
  };
};

struct Atom : public Object {
  // TODO: Use std::uint32_t instead of std::size_t?
  const char* value;
  const std::size_t byte_length;
  Atom(const char* value, const std::size_t byte_length)
      : value{value}, byte_length{byte_length} {};
  ~Atom() override {
    delete[] this->value;
  };
  std::size_t utf8length();
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
  std::uint8_t length;
  Value* values;
  Tuple(Value* values, std::uint8_t length) : length{length}, values{values} {};
  ~Tuple() override {
    delete[] this->values;
  };
  std::uint64_t byteSize() override {
    return sizeof(Tuple);
  };
};

struct RawFunction : public Object {
  std::uint16_t arity;
  const char* name;
  std::uint8_t* buffers;
  RawFunction(const char* name, std::uint16_t arity, std::uint8_t* buffers)
      : arity{arity}, name{name}, buffers{buffers} {};
  ~RawFunction() override {
    delete[] this->name;
    delete[] this->buffers;
  };
  std::uint64_t byteSize() override {
    return sizeof(RawFunction);
  };
};

class GC {
 private:
  const std::size_t maxNurserySize = 2048 * 2048;             //  ~4.2 MB
  const std::size_t maxRetirementHomeSize = 2048 * 2048 * 8;  // ~33.6 MB
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
  Value createAtom(const char* value, const std::size_t byte_length);
  Value createList(std::vector<Value> elements);
  Value createTable(std::unordered_map<std::string, Value> hashMap);
  Value createTuple(Value* values, std::uint8_t length);
  Value createRawFunction(const char* name,
                          std::uint16_t arity,
                          std::uint8_t* buffers);
};
}  // namespace flan
