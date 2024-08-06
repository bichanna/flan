#pragma once

#include <filesystem>
#include <unordered_map>
#include <vector>

#include "gc.hpp"

using namespace flan;
namespace fs = std::filesystem;

namespace flan {

const std::uint8_t VERSION[3] = {0, 0, 0};

const std::uint8_t MAGIC_NUMBER[4] = {0x46, 0x4C, 0x41, 0x4E};

struct ErrorInfo {
  std::uint16_t line;
  std::string lineText;
};

class VM {
 public:
  VM(fs::path fileName);
  ~VM();
  void run();

 private:
  char *buffer;
  std::vector<Value> stack;
  fs::path fileName;
  GC gc;
  std::vector<ErrorInfo> errorInfoList;
  std::unordered_map<std::string, Value> globals;

  void readErrorInfoSection();
  bool checkMagicNumber(std::uint8_t *bufferPtr);
  bool checkVersion(std::uint8_t *bufferPtr);
  std::uint8_t readUInt8(std::uint8_t *bufferPtr);
  std::uint16_t readUInt16(std::uint8_t *bufferPtr);
  std::uint32_t readUInt32(std::uint8_t *bufferPtr);

  void push(Value value);
  Value pop();

  void throwError(std::uint16_t errInfoIdx, std::string msg);
  void throwError(std::string msg);

  std::string readShortString(std::uint8_t *bufferPtr);
  Value readValue(std::uint8_t *bufferPtr);
  Value readInteger(std::uint8_t *bufferPtr);
  Value readFloat(std::uint8_t *bufferPtr);
  Value readBool(std::uint8_t *bufferPtr);
  Value readEmpty();
  Value readString(std::uint8_t *bufferPtr);
  Value readAtom(std::uint8_t *bufferPtr);

  Value performAdd(std::uint16_t errInfoIdx);
  Value performSub(std::uint16_t errInfoIdx);
  Value performMul(std::uint16_t errInfoIdx);
  Value performDiv(std::uint16_t errInfoIdx);
  Value performMod(std::uint16_t errInfoIdx);
  Value performEq(std::uint16_t errInfoIdx);
  Value performNEq(std::uint16_t errInfoIdx);
  Value performLT(std::uint16_t errInfoIdx);
  Value performLTE(std::uint16_t errInfoIdx);
  Value performGT(std::uint16_t errInfoIdx);
  Value performGTE(std::uint16_t errInfoIdx);
  Value performAnd();
  Value performOr();

  void jumpForward(std::uint8_t *bufferPtr, std::size_t offset);
};

enum class InstructionType : std::uint8_t {
  Load0,
  Load1,
  Load2,
  Load3,
  Load4,
  Load5,
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
  InitTable,
  InitTup,
  IdxListOrTup,
  GetTable,
  DefGlobal,
  GetGlobal,
  SetGlobal,
  Quit = 255,
};
}  // namespace flan
