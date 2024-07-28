mod decoder;
mod bit_reader;
mod lzw;

pub use decoder::Decoder;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum DisposalMethod {
    DoNotDispose = 1,
    RestoreToBackgroundColor = 2,
    RestoreToPrevious = 3,
}

impl DisposalMethod {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(DisposalMethod::DoNotDispose),
            2 => Some(DisposalMethod::RestoreToBackgroundColor),
            3 => Some(DisposalMethod::RestoreToPrevious),
            _ => None,
        }
    }
}
