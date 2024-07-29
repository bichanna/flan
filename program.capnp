@0xf8f48ebfa681b415;

struct Program {
  module          @0 :Module;
  importedModules @1 :List(Module);
}

struct Module {
  name         @0 :Text;
  instructions @1 :List(Instruction);
  errorInfo    @2 :List(ErrInfo);
}

struct ErrInfo {
  line  @0 :UInt16;
  colum @1 :UInt16;
  text  @2 :Text;
}

struct Instruction {
  errInfoIdx @0  :UInt32;
  union {
    push     @1  :List(Value);
    pop      @2  :UInt8;
    dup      @3  :Void;
    add      @4  :Void;
    sub      @5  :Void;
    mul      @6  :Void;
    div      @7  :Void;
    mod      @8  :Void;
    eq       @9  :Void;
    neq      @10 :Void;
    lt       @11 :Void;
    lte      @12 :Void;
    gt       @13 :Void;
    gte      @14 :Void;
    and      @15 :Void;
    or       @16 :Void;
    not      @17 :Void;
    jmp      @18 :UInt32;
    jz       @19 :UInt32;
    jnz      @20 :UInt32;
    initlist @21 :UInt16;
    initobj  @22 :UInt16;
  }
}

struct Value {
  union {
    integer @0 :Int64;
    float   @1 :Float64;
    string  @2 :Text;
    atom    @3 :Text;
    none    @4 :Void;
    empty   @5 :Void;
    boolean @6 :Bool;
  }
}
