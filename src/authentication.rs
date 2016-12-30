pub enum Method {
    NoAuthenticationRequired,
    UnknownMethod(u8),
    NoAcceptableMethods,
}

impl Method {
    pub fn new(code: u8) -> Method {
        match code {
            0x00 => Method::NoAuthenticationRequired,
            0xFF => Method::NoAcceptableMethods,
            _ => Method::UnknownMethod(code),
        }
    }

    pub fn code(&self) -> u8 {
        match *self {
            Method::NoAuthenticationRequired => 0,
            Method::UnknownMethod(code) => code,
            Method::NoAcceptableMethods => 0xFF,
        }
    }
}
