#include "vm.hpp"

#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <ios>
#include <iostream>
#include <sstream>
#include <variant>

#include "gc.hpp"

using namespace flan;

VM::VM(fs::path fileName) : gc{GC(&this->stack)} {
  auto inputStream = std::ifstream(fileName);
  this->fileName = fileName;

  if (!inputStream.is_open()) {
    std::stringstream ss;
    ss << "Failed to open file" << this->fileName;
    this->throwError(ss.str());
  }

  std::streamsize size = inputStream.tellg();
  inputStream.seekg(0, std::ios::beg);

  this->buffer = new char[size];

  if (!inputStream.read(buffer, size)) {
    std::stringstream ss;
    ss << "Failed to read file " << this->fileName;
    this->throwError(ss.str());
  }

  inputStream.close();

  this->readErrorInfoSection();
}

VM::~VM() {
  delete[] this->buffer;
}

void VM::readErrorInfoSection() {
  auto bufferPtr = reinterpret_cast<std::uint8_t*>(this->buffer);
  auto errorInfoListLength = this->readUInt16(bufferPtr);
  this->errorInfoList.reserve(errorInfoListLength);

  for (auto i = 0; i < errorInfoListLength; i++) {
    ErrorInfo errInfo;
    errInfo.line = this->readUInt16(bufferPtr);

    auto length = this->readUInt16(bufferPtr);
    std::string lineText;
    lineText.reserve(length);
    for (auto i = 0; i < length; i++)
      lineText += static_cast<char>(this->readUInt8(bufferPtr));
    errInfo.lineText = lineText;

    this->errorInfoList.push_back(errInfo);
  }
}

void VM::run() {
  auto bufferPtr = reinterpret_cast<std::uint8_t*>(this->buffer);

  bool quit = false;

  if (this->checkMagicNumber(bufferPtr)) {
    this->throwError("Invalid Magic number");
  }
  if (this->checkVersion(bufferPtr)) {
    this->throwError("Please update the Flan runtime");
  }

  do {
    auto instType = static_cast<InstructionType>(*bufferPtr);

    switch (instType) {
      case InstructionType::Push: {
        bufferPtr++;
        auto length = this->readUInt8(bufferPtr);
        for (auto i = 0; i < length; i++)
          this->push(this->readValue(bufferPtr));
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

      case InstructionType::Add:
        this->push(this->performAdd(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Sub:
        this->push(this->performSub(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Mul:
        this->push(this->performMul(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Div:
        this->push(this->performDiv(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Mod:
        this->push(this->performMod(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Eq:
        this->push(this->performEq(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::NEq:
        this->push(this->performNEq(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::LT:
        this->push(this->performLT(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::LTE:
        this->push(this->performLTE(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::GT:
        this->push(this->performLTE(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::GTE:
        this->push(this->performGTE(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::And:
        this->push(this->performAnd());
        break;

      case InstructionType::Or:
        this->push(this->performOr());
        break;

      case InstructionType::Quit:
        quit = true;
        break;

      default: {
        std::stringstream ss;
        ss << "Invalid instruction " << std::hex << std::setw(2)
           << std::setfill('0') << static_cast<std::uint8_t>(instType);
        this->throwError(ss.str());
        break;
      }
    }

    bufferPtr++;
  } while (!quit);
}

Value VM::performAdd(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<Object*>(left.value)) {
    auto leftObj = std::get<Object*>(left.value);
    if (typeid(leftObj) == typeid(String)) {
      auto l = static_cast<String*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(String)) {
          auto r = static_cast<String*>(rightObj);
          return new String(l->value + r->value);
        }
      }
    }
  } else if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l + r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) + r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l + static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l + r;
    }
  }

  std::stringstream ss;
  ss << "Cannot add " << left.toDbgString() << " and " << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  // Won't run
  return left;
}

Value VM::performSub(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l - r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) - r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l - static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l - r;
    }
  }

  std::stringstream ss;
  ss << "Cannot subtract " << right.toDbgString() << " from "
     << left.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  // Won't run
  return left;
}

Value VM::performMul(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l * r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) * r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l * static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l * r;
    }
  }

  std::stringstream ss;
  ss << "Cannot multipy " << left.toDbgString() << " by "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  // Won't run
  return left;
}

Value VM::performDiv(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      if (r == 0) this->throwError(errInfoIdx, "Cannot divide by zero");
      return l / r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      if (r == 0.0) this->throwError(errInfoIdx, "Cannot divide by zero");
      return static_cast<double>(l) / r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      if (r == 0) this->throwError(errInfoIdx, "Cannot divide by zero");
      return l / static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      if (r == 0.0) this->throwError(errInfoIdx, "Cannot divide by zero");
      return l / r;
    }
  }

  std::stringstream ss;
  ss << "Cannot divide " << left.toDbgString() << " by " << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  // Won't run
  return left;
}

Value VM::performMod(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      if (r == 0) this->throwError(errInfoIdx, "Cannot mod by 0");
      return l % r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      if (r == 0.0) this->throwError(errInfoIdx, "Cannot mod by 0");
      return fmod(static_cast<double>(l), r);
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      if (r == 0) this->throwError(errInfoIdx, "Cannot mod by 0");
      return fmod(l, static_cast<double>(r));
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      if (r == 0.0) this->throwError(errInfoIdx, "Cannot mod by 0");
      return fmod(l, r);
    }
  }

  std::stringstream ss;
  ss << "Cannot mod with " << left.toDbgString() << " and "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  // Won't run
  return left;
}

Value VM::performEq(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<char>(left.value)) {
    auto l = std::get<char>(left.value);
    if (l == 0) {
      return true;
    } else if (l == 1) {
      return std::holds_alternative<char>(right.value);
    }
  } else if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l == r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) == r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l == r;
    } else if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l == static_cast<double>(r);
    }
  } else if (std::holds_alternative<bool>(left.value)) {
    auto l = std::get<bool>(left.value);
    if (std::holds_alternative<bool>(right.value)) {
      auto r = std::get<bool>(right.value);
      return l == r;
    }
  } else if (std::holds_alternative<Object*>(left.value)) {
    auto leftObj = std::get<Object*>(left.value);
    if (typeid(leftObj) == typeid(String)) {
      auto l = static_cast<String*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(String)) {
          auto r = static_cast<String*>(rightObj);
          return l->value == r->value;
        }
      }
    } else if (typeid(leftObj) == typeid(Atom)) {
      auto l = static_cast<Atom*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(Atom)) {
          auto r = static_cast<Atom*>(rightObj);
          return l->value == r->value;
        }
      }
    }
  }

  std::stringstream ss;
  ss << "Cannot compare " << left.toDbgString() << " and "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  return left;
}

Value VM::performNEq(std::uint16_t errInfoIdx) {
  return !std::get<bool>(this->performEq(errInfoIdx).value);
}

Value VM::performLT(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<char>(left.value)) {
    auto l = std::get<char>(left.value);
    if (l == 0) return true;
  } else if (std::holds_alternative<char>(right.value)) {
    auto r = std::get<char>(right.value);
    if (r == 0) return true;
  } else if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l < r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) < r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l < static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l < r;
    }
  } else if (std::holds_alternative<Object*>(left.value)) {
    auto leftObj = std::get<Object*>(left.value);
    if (typeid(leftObj) == typeid(String)) {
      auto l = static_cast<String*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(String)) {
          auto r = static_cast<String*>(rightObj);
          return l->value < r->value;
        }
      }
    }
  }

  std::stringstream ss;
  ss << "Cannot compare " << left.toDbgString() << " and "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  return left;
}

Value VM::performLTE(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<char>(left.value)) {
    auto l = std::get<char>(left.value);
    return l == 0;
  } else if (std::holds_alternative<char>(right.value)) {
    auto r = std::get<char>(right.value);
    return r == 0;
  } else if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l <= r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) <= r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l <= static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l <= r;
    }
  } else if (std::holds_alternative<Object*>(left.value)) {
    auto leftObj = std::get<Object*>(left.value);
    if (typeid(leftObj) == typeid(String)) {
      auto l = static_cast<String*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(String)) {
          auto r = static_cast<String*>(rightObj);
          return l->value <= r->value;
        }
      }
    }
  }

  std::stringstream ss;
  ss << "Cannot compare " << left.toDbgString() << " and "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  return left;
}

Value VM::performGT(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<char>(left.value)) {
    auto l = std::get<char>(left.value);
    return l == 0;
  } else if (std::holds_alternative<char>(right.value)) {
    auto r = std::get<char>(right.value);
    return r == 0;
  } else if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l > r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) > r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l > static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l > r;
    }
  } else if (std::holds_alternative<Object*>(left.value)) {
    auto leftObj = std::get<Object*>(left.value);
    if (typeid(leftObj) == typeid(String)) {
      auto l = static_cast<String*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(String)) {
          auto r = static_cast<String*>(rightObj);
          return l->value > r->value;
        }
      }
    }
  }

  std::stringstream ss;
  ss << "Cannot compare " << left.toDbgString() << " and "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  return left;
}

Value VM::performGTE(std::uint16_t errInfoIdx) {
  auto right = this->pop();
  auto left = this->pop();

  if (std::holds_alternative<char>(left.value)) {
    auto l = std::get<char>(left.value);
    if (l == 0) return true;
  } else if (std::holds_alternative<char>(right.value)) {
    auto r = std::get<char>(right.value);
    if (r == 0) return true;
  } else if (std::holds_alternative<std::int64_t>(left.value)) {
    auto l = std::get<std::int64_t>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l >= r;
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return static_cast<double>(l) >= r;
    }
  } else if (std::holds_alternative<double>(left.value)) {
    auto l = std::get<double>(left.value);
    if (std::holds_alternative<std::int64_t>(right.value)) {
      auto r = std::get<std::int64_t>(right.value);
      return l >= static_cast<double>(r);
    } else if (std::holds_alternative<double>(right.value)) {
      auto r = std::get<double>(right.value);
      return l >= r;
    }
  } else if (std::holds_alternative<Object*>(left.value)) {
    auto leftObj = std::get<Object*>(left.value);
    if (typeid(leftObj) == typeid(String)) {
      auto l = static_cast<String*>(leftObj);
      if (std::holds_alternative<Object*>(right.value)) {
        auto rightObj = std::get<Object*>(right.value);
        if (typeid(rightObj) == typeid(String)) {
          auto r = static_cast<String*>(rightObj);
          return l->value >= r->value;
        }
      }
    }
  }

  std::stringstream ss;
  ss << "Cannot compare " << left.toDbgString() << " and "
     << right.toDbgString();
  this->throwError(errInfoIdx, ss.str());

  return left;
}

Value VM::performAnd() {
  auto right = this->pop();
  auto left = this->pop();
  return left.truthy() && right.truthy();
}

Value VM::performOr() {
  auto right = this->pop();
  auto left = this->pop();
  return left.truthy() || right.truthy();
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
  return static_cast<std::uint16_t>(low_byte) |
         (static_cast<std::uint16_t>(high_byte) << 8);
}

std::uint32_t VM::readUInt32(std::uint8_t* bufferPtr) {
  auto byte1 = this->readUInt8(bufferPtr);
  auto byte2 = this->readUInt8(bufferPtr);
  auto byte3 = this->readUInt8(bufferPtr);
  auto byte4 = this->readUInt8(bufferPtr);
  return static_cast<std::uint32_t>(byte1) |
         (static_cast<std::uint32_t>(byte2)) |
         (static_cast<std::uint32_t>(byte3)) |
         (static_cast<std::uint32_t>(byte4));
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
      this->throwError(ss.str());
    }
  }

  // Won't run
  return Value();
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

void VM::throwError(std::uint16_t errInfoIdx, std::string msg) {
  delete[] this->buffer;

  ErrorInfo errInfo = this->errorInfoList.at(errInfoIdx);
  std::cerr << errInfo.lineText << "\n";
  std::cerr << "Error at line " << errInfo.line << ":" << msg << std::endl;
  std::exit(1);
}

void VM::throwError(std::string msg) {
  delete[] this->buffer;

  std::cerr << "Error: " << msg << std::endl;
  std::exit(1);
}
