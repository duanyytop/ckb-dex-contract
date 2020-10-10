use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    // Add customized errors here...
    WrongPubkey,
    LoadPrefilledData,
    RecoverPubkey,
    WrongDataLengthOrFormat,
    WrongSUDTAmount,
    WrongOrderType,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            WrongDataLengthOrFormat => Self::WrongDataLengthOrFormat,
            WrongSUDTAmount => Self::WrongSUDTAmount,
            WrongOrderType => Self::WrongOrderType,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}
