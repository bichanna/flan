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
  virtual ~Object() {};
  virtual void mark();
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
  void mark() override;
  std::uint64_t byteSize() override {
    return sizeof(List);
  };
};

struct Table : public Object {
  std::unordered_map<std::string, Value> hashMap;
  Table(std::unordered_map<std::string, Value> hashMap) : hashMap{hashMap} {};
  ~Table() override {};
  void mark() override;
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
  void mark() override;
  std::uint64_t byteSize() override {
    return sizeof(Tuple);
  };
};

struct Function : public Object {
  std::uint16_t arity;
  const char* name;
  std::uint8_t* buffers;
  Function(const char* name, std::uint16_t arity, std::uint8_t* buffers)
      : arity{arity}, name{name}, buffers{buffers} {};
  ~Function() override {
    delete[] this->name;
    delete[] this->buffers;
  };
  std::uint64_t byteSize() override {
    return sizeof(Function);
  };
};

struct Upvalue : public Object {
  Value value;
  Upvalue(Value value) : value{value} {};
  void mark() override;
  ~Upvalue() override {};
  std::uint64_t byteSize() override {
    return sizeof(Upvalue);
  }
};

struct Closure : public Object {
  uint8_t upvalueCount;
  Upvalue** upvalues;
  Function* function;
  Closure(Function* function, Upvalue** upvalues, uint8_t upvalueCount)
      : upvalueCount{upvalueCount}, upvalues{upvalues}, function{function} {};
  ~Closure() override {
    delete this->function;
    delete[] this->upvalues;
  };
  void mark() override;
  std::uint64_t byteSize() override {
    return sizeof(Closure);
  }
};

class GC {
 private:
  const std::size_t maxNurserySize = 2048 * 2048 * 2;          //  ~8.4 MB
  const std::size_t maxRetirementHomeSize = 2048 * 2048 * 16;  // ~67.1 MB
  std::vector<Value>* stack;

  std::size_t retirementHomeHeap = 0;
  std::forward_list<Object*> retirementHome;

  std::size_t nurseryHeap = 0;
  std::forward_list<Object*> nursery;

  void GCNursery();
  void GCRetirementHome();

  void mayGC();

  void addToNursery(Object* obj);
  void addToRetirementHome(Object* obj);
  void removeFromNursery(Object* obj);
  void removeFromRetirementHome(Object* obj);

 public:
  GC(std::vector<Value>* stack) : stack{stack} {};
  Value createString(std::string value);
  Value createAtom(const char* value, const std::size_t byte_length);
  Value createList(std::vector<Value> elements);
  Value createTable(std::unordered_map<std::string, Value> hashMap);
  Value createTuple(Value* values, std::uint8_t length);
  Value createFunction(const char* name,
                       std::uint16_t arity,
                       std::uint8_t* buffers);
  Value createUpvalue(Value vlaue);
  Upvalue* createUpvaluePtr(Value value);
  Value createClosure(Function* Function,
                      Upvalue** upvalues,
                      std::uint8_t upvalueCount);
};
}  // namespace flan
