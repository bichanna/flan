#include "vm.hpp"

#include <cstdint>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <ios>
#include <stdexcept>

#include "gc.hpp"

using namespace impala;

VM::VM(fs::path fileName) : gc{GC(&this->stack)} {
  this->inputStream = std::ifstream(fileName);
  this->fileName = fileName;
  if (!this->inputStream.is_open()) {
    std::stringstream ss;
    ss << "Failed to open file" << fileName;
    throw std::runtime_error(ss.str());
  }
}

void VM::run() {
  std::streamsize size = this->inputStream.tellg();
  this->inputStream.seekg(0, std::ios::beg);

  char* buffer = new char[size];
  if (!this->inputStream.read(buffer, size)) {
    delete[] buffer;
    std::stringstream ss;
    ss << "Failed to read file" << this->fileName;
    throw std::runtime_error(ss.str());
  }

  inputStream.close();

  std::uint8_t* bufferPtr = (std::uint8_t*)buffer;

  if (this->checkMagicNumber(bufferPtr)) {
    // TODO: Throw error
  }
  if (this->checkVersion(bufferPtr)) {
    // TODO: Throw error
  }

  do {
    auto instType = static_cast<InstructionType>(*bufferPtr);

    switch (instType) {
      case InstructionType::Push: {
        bufferPtr++;
        auto length = this->readUInt8(bufferPtr);
        for (auto i = 0; i < length; i++) {
          try {
            this->push(this->readValue(bufferPtr));
          } catch (const std::exception& e) {
            // TODO: Throw error
          }
        }
        break;
      }

      case InstructionType::Pop: {
        bufferPtr++;
        this->pop();
        break;
      }

      case InstructionType::PopN: {
        bufferPtr++;
        auto length = this->readUInt8(bufferPtr);
        for (auto i = 0; i < length; i++) this->pop();
        break;
      }

      case InstructionType::Dup: {
        bufferPtr++;
        auto value = this->stack.back();
        this->push(value);
        break;
      }

      case InstructionType::Add: {
        this->push(this->performAdd(bufferPtr));
        break;
      }

      case InstructionType::Sub: {
        this->push(this->performSub(bufferPtr));
        break;
      }

      case InstructionType::Mul: {
        this->push(this->performMul(bufferPtr));
        break;
      }

      case InstructionType::Div: {
        this->push(this->performDiv(bufferPtr));
        break;
      }

      case InstructionType::Mod: {
        this->push(this->performMod(bufferPtr));
        break;
      }

      case InstructionType::Eq: {
        this->push(this->performEq(bufferPtr));
        break;
      }

      case InstructionType::NEq: {
        this->push(this->performNEq(bufferPtr));
        break;
      }

      case InstructionType::LT: {
        this->push(this->performLT(bufferPtr));
        break;
      }

      case InstructionType::LTE: {
        this->push(this->performLTE(bufferPtr));
        break;
      }

      case InstructionType::GT: {
        this->push(this->performLTE(bufferPtr));
        break;
      }

      case InstructionType::GTE: {
        this->push(this->performGTE(bufferPtr));
        break;
      }

      case InstructionType::And: {
        this->push(this->performAnd(bufferPtr));
        break;
      }

      case InstructionType::Or: {
        this->push(this->performOr(bufferPtr));
        break;
      }

      default:
        break;
    }
  } while (bufferPtr++);

  delete[] buffer;
}

Value VM::performAdd(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left + right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performSub(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left - right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performMul(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left * right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performDiv(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left / right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performMod(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left % right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performEq(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left == right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performNEq(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left != right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performLT(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left < right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performLTE(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left <= right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performGT(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left > right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performGTE(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left >= right;
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performAnd(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left.truty() && right.truty();
  } catch (const std::exception& e) {
    throw;
  }
}

Value VM::performOr(std::uint8_t* bufferPtr) {
  auto right = this->pop();
  auto left = this->pop();
  try {
    return left.truty() || right.truty();
  } catch (const std::exception& e) {
    throw;
  }
}

bool VM::checkMagicNumber(std::uint8_t* bufferPtr) {
  return (this->readUInt8(bufferPtr) == MAGIC_NUMBER[0]) &&
         (this->readUInt8(bufferPtr) == MAGIC_NUMBER[1]) &&
         (this->readUInt8(bufferPtr) == MAGIC_NUMBER[2]) &&
         (this->readUInt8(bufferPtr) == MAGIC_NUMBER[3]);
}

bool VM::checkVersion(std::uint8_t* bufferPtr) {
  return (this->readUInt8(bufferPtr) == VERSION[0]) &&
         (this->readUInt8(bufferPtr) == VERSION[1]) &&
         (this->readUInt8(bufferPtr) <= VERSION[2]);
}

std::uint8_t VM::readUInt8(std::uint8_t* bufferPtr) {
  std::uint8_t value = *bufferPtr;
  bufferPtr++;
  return value;
}

std::uint16_t VM::readUInt16(std::uint8_t* bufferPtr) {
  auto low_byte = this->readUInt8(bufferPtr);
  auto high_byte = this->readUInt8(bufferPtr);
  return (std::uint16_t)low_byte | ((std::uint16_t)high_byte << 8);
}

std::uint32_t VM::readUInt32(std::uint8_t* bufferPtr) {
  auto byte1 = this->readUInt8(bufferPtr);
  auto byte2 = this->readUInt8(bufferPtr);
  auto byte3 = this->readUInt8(bufferPtr);
  auto byte4 = this->readUInt8(bufferPtr);
  return (std::uint32_t)byte1 | ((std::uint32_t)byte2) |
         ((std::uint32_t)byte3) | ((std::uint32_t)byte4);
}

void VM::push(Value value) {
  this->stack.push_back(value);
}

Value VM::pop() {
  auto popped = this->stack.back();
  this->stack.pop_back();
  return popped;
}

Value VM::readValue(std::uint8_t* bufferPtr) {
  auto type = this->readUInt8(bufferPtr);
  bufferPtr++;

  switch (type) {
    case 0:
      return this->readInteger(bufferPtr);
    case 1:
      return this->readFloat(bufferPtr);
    case 2:
      return this->readBool(bufferPtr);
    case 3:
      return this->readNone();
    case 4:
      return this->readEmpty();
    case 5:
      return Value(readString(bufferPtr));
    case 6:
      return Value(readAtom(bufferPtr));
    default: {
      std::stringstream ss;
      ss << "Invalid value type " << std::hex << std::setw(2)
         << std::setfill('0') << type;
      throw std::runtime_error(ss.str());
    }
  }
}

Value VM::readInteger(std::uint8_t* bufferPtr) {
  std::uint8_t bytes[4];
  for (auto i = 0; i < 4; i++) bytes[i] = this->readUInt8(bufferPtr);

  std::int64_t result = 0;
  for (auto i = 0; i < 4; i++)
    result |= static_cast<int64_t>(bytes[i]) << (i * 8);

  return result;
}

Value VM::readFloat(std::uint8_t* bufferPtr) {
  std::uint8_t bytes[4];
  for (auto i = 0; i < 4; i++) bytes[i] = this->readUInt8(bufferPtr);

  double result = 0.0;
  std::memcpy(&result, bytes, 4);

  return result;
}

Value VM::readBool(std::uint8_t* bufferPtr) {
  return this->readUInt8(bufferPtr) == 1;
}

Value VM::readNone() {
  Value v;
  v.value = 1;
  return v;
}

Value VM::readEmpty() {
  Value v;
  v.value = 0;
  return v;
}

String* VM::readString(std::uint8_t* bufferPtr) {
  auto length = this->readUInt16(bufferPtr);
  std::string s;
  s.reserve(length);
  for (auto i = 0; i < length; i++) s += (char)this->readUInt8(bufferPtr);
  return new String{s};
}

Atom* VM::readAtom(std::uint8_t* bufferPtr) {
  auto length = this->readUInt8(bufferPtr);
  std::string s;
  s.reserve(length);
  for (auto i = 0; i < length; i++) s += (char)this->readUInt8(bufferPtr);
  return new Atom{s};
}
