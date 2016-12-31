#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Command {
    Connect,
    Bind,
    UdpAssociate,
}
impl Command {
    pub fn new(code: u8) -> Option<Command> {
        match code {
            0x01 => Some(Command::Connect),
            0x02 => Some(Command::Bind),
            0x03 => Some(Command::UdpAssociate),
            _ => None,
        }
    }
    pub fn code(&self) -> u8 {
        match *self {
            Command::Connect => 0x01,
            Command::Bind => 0x02,
            Command::UdpAssociate => 0x03,
        }
    }
}

