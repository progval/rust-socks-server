use authentication::Method;
use request_error::RequestError;
use command::Command;
use address::Address;

pub struct InitialMessage {
    pub version: u8,
    pub methods: Vec<Method>
}

impl InitialMessage {
    pub fn decode(bytes: &[u8]) -> Result<InitialMessage, String> {
        match (bytes.get(0), bytes.get(1)) {
            (Some(&version), Some(&nmethods)) => {
                let nmethods = nmethods as usize;
                if bytes.len() == nmethods+2 {
                    let methods = bytes[2..nmethods+2].iter().map(|b| *b).map(Method::new).collect();
                    Ok(InitialMessage { version: version, methods: methods })
                }
                else {
                    Err(format!("Packet length ({}) does not match nmethods+2 ({}). Received these bytes: {:?}", bytes.len(), nmethods+2, bytes))
                }
            },
            _ => Err("Packet is too short (< 2 bytes)".to_owned()),
        }
    }
}

pub struct InitialResponse {
    pub version: u8,
    pub method: Method,
}

impl InitialResponse {
    pub fn encode(&self) -> Vec<u8> {
        vec![self.version, self.method.code()]
    }
}



pub struct Request {
    pub version: u8,
    pub command: Command,
    pub dest_address: Address,
}

impl Request {
    pub fn decode(bytes: &[u8]) -> Result<Request, RequestError> {
        let (version, command_code) = 
            match (bytes.get(0), bytes.get(1)) {
                (Some(version), Some(command_code)) =>
                    (*version, *command_code),
                _ => return Err(RequestError::PacketTooShort),
            };
        let command = match Command::new(command_code) {
            Some(command) => command,
            None => return Err(RequestError::CommandNotSupported(command_code)),
        };
        let dest_address = try!(Address::decode(&bytes[3..]));
        Ok(Request {
            version: version,
            command: command,
            dest_address: dest_address,
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum ReplyType {
    Succeeded,
    GeneralFailure,
    ConnectionNotAllowed,
    NetworkUnreachable,
    HostUnreachable,
    ConnectionRefused,
    TTLExpired,
    CommandNotSupported,
    AddressTypeNotSupported,
}

impl ReplyType {
    pub fn code(&self) -> u8 {
        match *self {
            ReplyType::Succeeded => 0x00,
            ReplyType::GeneralFailure => 0x01,
            ReplyType::ConnectionNotAllowed => 0x02,
            ReplyType::NetworkUnreachable => 0x03,
            ReplyType::HostUnreachable => 0x04,
            ReplyType::ConnectionRefused => 0x05,
            ReplyType::TTLExpired => 0x06,
            ReplyType::CommandNotSupported => 0x07,
            ReplyType::AddressTypeNotSupported => 0x08,
        }
    }
}

pub struct Reply {
    pub version: u8,
    pub reply: ReplyType,
    pub bound_address: Address,
}

impl Reply {
    pub fn encode(&self) -> Vec<u8> {
        let address_bytes = self.bound_address.encode();
        let mut res = Vec::new();
        res.resize(address_bytes.len()+3, 0);
        res[0] = self.version;
        res[1] = self.reply.code();
        // res[2] is reserved
        res[3..3+address_bytes.len()].copy_from_slice(&address_bytes);
        res
    }
}
