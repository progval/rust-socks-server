#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RequestError {
    UnsupportedVersion(u8),
    CommandNotSupported(u8),
    AddressTypeNotSupported(u8),
    PacketTooShort,
}

