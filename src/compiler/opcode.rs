pub type Position = (usize, usize);

pub fn pos_str(pos: &Position) -> String {
    format!("{},{}", pos.0, pos.1)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Return,
    Constant,
    ConstantLong,
    Negate,
    Add,
    Sub,
    Mult,
    Div,
    Mod,
    DefineGlobalVar,
    GetGlobalVar,
    SetGlobalVar,
    Pop,
    PopN,
    DefineLocal,
    GetLocal,
    SetLocalVar,
    SetLocalList,
    SetLocalObj,
    InitList,
    InitObj,
}

impl OpCode {
    pub fn u8_to_opcode(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Return),
            1 => Some(Self::Constant),
            2 => Some(Self::ConstantLong),
            3 => Some(Self::Negate),
            4 => Some(Self::Add),
            5 => Some(Self::Sub),
            6 => Some(Self::Mult),
            7 => Some(Self::Div),
            8 => Some(Self::Mod),
            9 => Some(Self::DefineGlobalVar),
            10 => Some(Self::GetGlobalVar),
            11 => Some(Self::SetGlobalVar),
            12 => Some(Self::Pop),
            13 => Some(Self::PopN),
            14 => Some(Self::DefineLocal),
            15 => Some(Self::GetLocal),
            16 => Some(Self::SetLocalVar),
            17 => Some(Self::SetLocalList),
            18 => Some(Self::SetLocalObj),
            19 => Some(Self::InitList),
            20 => Some(Self::InitObj),
            _ => None,
        }
    }
}
