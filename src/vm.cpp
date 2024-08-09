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
#include <string>
#include <variant>

#include "gc.hpp"

using namespace flan;

VM::VM(fs::path fileName) : stack{}, gc{GC(this->stack.actualStack())} {
  this->callframes.reserve(CALL_FRAMES_MAX);

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

  if (!this->checkMagicNumber(bufferPtr)) {
    this->throwError("Invalid Magic number");
  }
  if (!this->checkVersion(bufferPtr)) {
    this->throwError("Update the Flan runtime");
  }

  for (;;) {
    auto instType = static_cast<InstructionType>(*bufferPtr);

    switch (instType) {
      case InstructionType::LoadNeg1:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(-1)));
        break;

      case InstructionType::Load0:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(0)));
        break;

      case InstructionType::Load1:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(1)));
        break;

      case InstructionType::Load2:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(2)));
        break;

      case InstructionType::Load3:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(3)));
        break;

      case InstructionType::Load4:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(4)));
        break;

      case InstructionType::Load5:
        bufferPtr++;
        this->push(Value(static_cast<std::int64_t>(5)));
        break;

      case InstructionType::Load:
        bufferPtr++;
        this->push(this->readValue(bufferPtr));
        break;

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

      case InstructionType::Nip: {
        bufferPtr++;
        auto last = this->pop();
        this->pop();
        this->push(last);
        break;
      }

      case InstructionType::NipN: {
        bufferPtr++;
        auto length = this->readUInt8(bufferPtr);
        auto last = this->pop();
        for (auto i = 0; i < length; i++) this->pop();
        this->push(last);
        break;
      }

      case InstructionType::Dup: {
        bufferPtr++;
        auto value = this->stack.last();
        this->push(value);
        break;
      }

      case InstructionType::Add:
        bufferPtr++;
        this->push(this->performAdd(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Sub:
        bufferPtr++;
        this->push(this->performSub(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Mul:
        bufferPtr++;
        this->push(this->performMul(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Div:
        bufferPtr++;
        this->push(this->performDiv(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Mod:
        bufferPtr++;
        this->push(this->performMod(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::Eq:
        bufferPtr++;
        this->push(this->performEq(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::NEq:
        bufferPtr++;
        this->push(this->performNEq(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::LT:
        bufferPtr++;
        this->push(this->performLT(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::LTE:
        bufferPtr++;
        this->push(this->performLTE(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::GT:
        bufferPtr++;
        this->push(this->performLTE(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::GTE:
        bufferPtr++;
        this->push(this->performGTE(this->readUInt16(bufferPtr)));
        break;

      case InstructionType::And:
        bufferPtr++;
        this->push(this->performAnd());
        break;

      case InstructionType::Or:
        bufferPtr++;
        this->push(this->performOr());
        break;

      case InstructionType::Not: {
        bufferPtr++;
        Value& last = this->stack.last();
        last.value = !last.truthy();
        break;
      }

      case InstructionType::Negate: {
        bufferPtr++;
        auto value = this->pop();
        if (std::holds_alternative<std::int64_t>(value.value)) {
          auto integer = std::get<std::int64_t>(value.value);
          this->push(-integer);
        } else if (std::holds_alternative<double>(value.value)) {
          auto floatNum = std::get<double>(value.value);
          this->push(-floatNum);
        }
        break;
      }

      case InstructionType::Jmp: {
        bufferPtr++;
        this->jumpForward(bufferPtr, this->readUInt32(bufferPtr));
        break;
      }

      case InstructionType::Jz: {
        bufferPtr++;
        auto offset = this->readUInt32(bufferPtr);
        if (!this->pop().truthy()) this->jumpForward(bufferPtr, offset);
        break;
      }

      case InstructionType::Jnz: {
        bufferPtr++;
        auto offset = this->readUInt32(bufferPtr);
        if (this->pop().truthy()) this->jumpForward(bufferPtr, offset);
        break;
      }

      case InstructionType::InitList: {
        bufferPtr++;
        auto length = this->readUInt32(bufferPtr);
        std::vector<Value> elements;
        elements.reserve(length);
        for (std::uint32_t i = 0; i < length; i++)
          elements.push_back(this->pop());
        this->push(this->gc.createList(std::move(elements)));
        break;
      }

      case InstructionType::InitTable: {
        bufferPtr++;
        auto length = this->readUInt32(bufferPtr);
        std::unordered_map<std::string, Value> hashMap;
        hashMap.reserve(length);

        for (std::uint32_t i = 0; i < length; i++) {
          auto key = this->readShortString(bufferPtr);
          hashMap[key] = this->pop();
        }

        this->push(this->gc.createTable(hashMap));

        break;
      }

      case InstructionType::InitTup: {
        bufferPtr++;
        auto length = this->readUInt32(bufferPtr);
        std::vector<Value> values;
        values.reserve(length);
        for (std::uint32_t i = 0; i < length; i++)
          values.push_back(this->pop());
        this->push(this->gc.createTuple(std::move(values)));
        break;
      }

      case InstructionType::IdxListOrTup: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto idx = std::get<std::int64_t>(this->readInteger(bufferPtr).value);
        auto value = this->pop();

        if (!std::holds_alternative<Object*>(value.value)) {
          std::stringstream ss;
          ss << "Expected a list or tuple but got " << value.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto obj = std::get<Object*>(value.value);

        std::vector<Value> values;
        if (typeid(obj) == typeid(List)) {
          values = static_cast<List*>(obj)->elements;
        } else if (typeid(obj) == typeid(Tuple)) {
          values = static_cast<Tuple*>(obj)->values;
        } else {
          std::stringstream ss;
          ss << "Expected a list or tuple but got " << value.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        if (values.size() <= static_cast<std::uint64_t>(idx))
          this->throwError(errInfoIdx, "Index out of range");

        if ((idx < 0) && (0 <= static_cast<std::int64_t>(values.size() - idx)))
          this->push(values.at(values.size() - idx));
        else
          this->push(values.at(idx));

        break;
      }

      case InstructionType::SetList: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto idx = std::get<std::int64_t>(this->readInteger(bufferPtr).value);
        auto newValue = this->pop();
        auto couldBeList = this->pop();

        if (!std::holds_alternative<Object*>(couldBeList.value)) {
          std::stringstream ss;
          ss << "Expected a list but got " << couldBeList.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto obj = std::get<Object*>(couldBeList.value);
        if (typeid(obj) != typeid(List)) {
          std::stringstream ss;
          ss << "Expected a list but got " << couldBeList.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto elements = static_cast<List*>(obj)->elements;
        if (elements.size() <= static_cast<std::uint64_t>(idx))
          this->throwError(errInfoIdx, "Index out of range");

        if ((idx < 0) &&
            (0 <= static_cast<std::int64_t>(elements.size() - idx)))
          elements[elements.size() - idx] = newValue;
        else
          elements[idx] = newValue;

        break;
      }

      case InstructionType::GetMember: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto key = this->readShortString(bufferPtr);
        auto value = this->pop();

        if (!std::holds_alternative<Object*>(value.value)) {
          std::stringstream ss;
          ss << "Expected a table but got " << value.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto obj = std::get<Object*>(value.value);
        if (typeid(obj) != typeid(Table)) {
          std::stringstream ss;
          ss << "Expected a table but got " << value.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto table = static_cast<Table*>(obj);
        if (!table->hashMap.count(key)) {
          std::stringstream ss;
          ss << "Table does not have key " << value.toDbgString();
          this->throwError(ss.str());
        }

        this->push(table->hashMap[key]);

        break;
      }

      case InstructionType::SetMember: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto key = this->readShortString(bufferPtr);
        auto newValue = this->pop();
        auto couldBeTable = this->pop();

        if (!std::holds_alternative<Object*>(couldBeTable.value)) {
          std::stringstream ss;
          ss << "Expected a table but got " << couldBeTable.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto obj = std::get<Object*>(couldBeTable.value);
        if (typeid(obj) != typeid(Table)) {
          std::stringstream ss;
          ss << "Expected a table but got " << couldBeTable.toDbgString();
          this->throwError(errInfoIdx, ss.str());
        }

        auto table = static_cast<Table*>(obj);
        table->hashMap.insert_or_assign(key, newValue);

        break;
      }

      case InstructionType::DefGlobal: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto varName = this->readShortString(bufferPtr);
        auto value = this->pop();

        if (this->globals.count(varName)) {
          std::stringstream ss;
          ss << "Global variable '" << varName << "' is already defined";
          this->throwError(errInfoIdx, ss.str());
        } else {
          this->globals.insert({varName, value});
        }

        break;
      }

      case InstructionType::GetGlobal: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto varName = this->readShortString(bufferPtr);

        if (!this->globals.count(varName)) {
          std::stringstream ss;
          ss << "Global variable '" << varName << "' is not defined";
          this->throwError(errInfoIdx, ss.str());
        } else {
          this->push(this->globals[varName]);
        }

        break;
      }

      case InstructionType::SetGlobal: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto varName = this->readShortString(bufferPtr);
        auto value = this->pop();

        if (!this->globals.count(varName)) {
          std::stringstream ss;
          ss << "Global variable '" << varName << "' is not defined";
          this->throwError(errInfoIdx, ss.str());
        } else {
          this->globals.insert_or_assign(varName, value);
        }

        break;
      }

      case InstructionType::GetLocal: {
        bufferPtr++;
        auto idx = this->readUInt16(bufferPtr);
        push(this->stack[idx]);
        break;
      }

      case InstructionType::SetLocal: {
        bufferPtr++;
        auto idx = this->readUInt16(bufferPtr);
        this->stack[idx] = this->stack.last();
        break;
      }

      case InstructionType::CallFn: {
        bufferPtr++;
        auto errInfoIdx = this->readUInt16(bufferPtr);
        auto argCount = this->readUInt16(bufferPtr);
        auto couldBeFunc = this->stack.fromLast(argCount - 1);
        this->callFunc(bufferPtr, couldBeFunc, argCount, errInfoIdx);
        break;
      }

      case InstructionType::RetFn: {
        bufferPtr++;

        auto poppedFrame = this->callframes.back();
        this->callframes.pop_back();

        bufferPtr = poppedFrame.retAddr;
        this->stack.from = poppedFrame.prevFrom;

        break;
      }

      case InstructionType::Halt:
        bufferPtr++;
        goto quitRun;
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
  }

quitRun:
  return;
}

void VM::callFunc(std::uint8_t* bufferPtr,
                  Value couldBeFunc,
                  std::uint16_t argCount,
                  std::uint16_t errInfoIdx) {
  if (!std::holds_alternative<Object*>(couldBeFunc.value)) {
    std::stringstream ss;
    ss << couldBeFunc.toDbgString() << " is not callable";
    this->throwError(errInfoIdx, ss.str());
  }

  auto obj = std::get<Object*>(couldBeFunc.value);
  if (typeid(obj) != typeid(Function)) {
    std::stringstream ss;
    ss << couldBeFunc.toDbgString() << " is not callable";
    this->throwError(errInfoIdx, ss.str());
  }

  auto func = static_cast<Function*>(obj);

  if (func->arity != argCount) {
    std::stringstream ss;
    ss << couldBeFunc.toDbgString() << " takes " << func->arity
       << " arguments but " << argCount << " was given";
    this->throwError(errInfoIdx, ss.str());
  }

  auto frame = CallFrame(bufferPtr, func, this->stack.from);
  this->callframes.push_back(frame);
  this->stack.setFrom(argCount);
  bufferPtr = func->buffers;
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
          return this->gc.createString(l->value + r->value);
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
    return true;
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
    return true;
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
    return true;
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
    return true;
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
    return true;
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
         (this->readUInt8(bufferPtr) <= VERSION[1]) &&
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
  this->stack.push(value);
}

Value VM::pop() {
  return this->stack.pop();
}

void VM::jumpForward(std::uint8_t* bufferPtr, std::size_t offset) {
  bufferPtr += offset;
}

std::string VM::readShortString(std::uint8_t* bufferPtr) {
  auto length = this->readUInt8(bufferPtr);
  std::string str;
  str.reserve(length);
  for (std::uint32_t i = 0; i < length; i++)
    str += static_cast<char>(this->readUInt8(bufferPtr));
  return str;
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
      return this->readEmpty();
    case 4:
      return readString(bufferPtr);
    case 5:
      return readAtom(bufferPtr);
    case 6:
      return readFunction(bufferPtr);
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

Value VM::readEmpty() {
  return Value();
}

Value VM::readString(std::uint8_t* bufferPtr) {
  auto length = this->readUInt16(bufferPtr);
  std::string s;
  s.reserve(length);
  for (auto i = 0; i < length; i++)
    s += static_cast<char>(this->readUInt8(bufferPtr));
  return this->gc.createString(s);
}

Value VM::readAtom(std::uint8_t* bufferPtr) {
  auto length = this->readUInt8(bufferPtr);
  std::string s;
  s.reserve(length);
  for (auto i = 0; i < length; i++)
    s += static_cast<char>(this->readUInt8(bufferPtr));
  return this->gc.createAtom(s);
}

Value VM::readFunction(std::uint8_t* bufferPtr) {
  auto funcName = this->readShortString(bufferPtr);
  auto arity = this->readUInt16(bufferPtr);
  auto funcBuffers = this->readFunctionBody(bufferPtr);
  return this->gc.createFunction(funcName, arity, funcBuffers);
}

std::uint8_t* VM::readFunctionBody(std::uint8_t* bufferPtr) {
  auto length = std::get<std::int64_t>(this->readInteger(bufferPtr).value);
  auto buffers = new std::uint8_t[length];
  for (auto i = 0; i < length; i++) buffers[i] = this->readUInt8(bufferPtr);

  auto endFn = this->readUInt8(bufferPtr);
  if (InstructionType::EndFn != static_cast<InstructionType>(endFn)) {
    std::stringstream ss;
    ss << "Expected " << std::hex << std::setw(2) << std::setfill('0')
       << static_cast<std::uint8_t>(InstructionType::EndFn) << " but got "
       << std::hex << std::setw(2) << std::setfill('0') << endFn;
    this->throwError(ss.str());
  }

  return buffers;
}

void VM::throwError(std::uint16_t errInfoIdx, std::string msg) {
  delete[] this->buffer;

  std::cerr << "Stack trace:\n";

  // TODO: Maybe fix this later?
  for (int i = this->callframes.size() - 1; i >= 0; i--) {
    auto frame = this->callframes[i];
    std::cerr << frame.function->name << "\n";
  }

  std::cerr << "\n";

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

Stack::Stack() {
  this->stack.reserve(CALL_FRAMES_MAX * UINT8_MAX);
  this->from = 0;
}

Value& Stack::last() {
  return this->stack.back();
}

void Stack::push(Value value) {
  this->stack.push_back(value);
}

Value Stack::pop() {
  auto popped = this->stack.back();
  this->stack.pop_back();
  return popped;
}

Value& Stack::operator[](std::uint64_t index) {
  return this->stack[this->from + index];
}

Value& Stack::fromLast(std::uint64_t indexFromLast) {
  return this->stack[this->stack.size() - indexFromLast];
}

void Stack::setFrom(std::uint16_t argCount) {
  this->from = this->stack.size() - argCount - 1;
}

std::vector<Value>* Stack::actualStack() {
  return &this->stack;
}
