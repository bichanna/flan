pub type Position = (usize, usize);

pub fn pos_str(pos: &Position) -> String {
    format!("{},{}", pos.0, pos.1)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Return,
    Constant,
}

impl OpCode {
    pub fn u8_to_opcode(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Return),
            1 => Some(Self::Constant),
            _ => None,
        }
    }
}
