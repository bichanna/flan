#pragma once

#include <filesystem>
#include <vector>

#include "gc.hpp"

using namespace impala;
namespace fs = std::filesystem;

namespace impala {

const std::uint8_t VERSION[3] = {0, 0, 0};

const std::uint8_t MAGIC_NUMBER[4] = {0x49, 0x4D, 0x50, 0x41};

class VM {
 public:
  VM(fs::path fileName);
  ~VM();

 private:
  char *buffer;
  std::vector<Value> stack;
  fs::path fileName;
  GC gc;

  void run();
  bool checkMagicNumber(std::uint8_t *bufferPtr);
  bool checkVersion(std::uint8_t *bufferPtr);
  std::uint8_t readUInt8(std::uint8_t *bufferPtr);
  std::uint16_t readUInt16(std::uint8_t *bufferPtr);
  std::uint32_t readUInt32(std::uint8_t *bufferPtr);

  void push(Value value);
  Value pop();

  Value readValue(std::uint8_t *bufferPtr);
  Value readInteger(std::uint8_t *bufferPtr);
  Value readFloat(std::uint8_t *bufferPtr);
  Value readBool(std::uint8_t *bufferPtr);
  Value readNone();
  Value readEmpty();
  String *readString(std::uint8_t *bufferPtr);
  Atom *readAtom(std::uint8_t *bufferPtr);

  Value performAdd();
  Value performSub();
  Value performMul();
  Value performDiv();
  Value performMod();
  Value performEq();
  Value performNEq();
  Value performLT();
  Value performLTE();
  Value performGT();
  Value performGTE();
  Value performAnd();
  Value performOr();
};

enum class InstructionType : std::uint8_t {
  Push,
  Pop,
  PopN,
  Dup,
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Eq,
  NEq,
  LT,
  LTE,
  GT,
  GTE,
  And,
  Or,
  Not,
  Jmp,
  Jz,
  Jnz,
  InitList,
  InitObj,
};
}  // namespace impala
