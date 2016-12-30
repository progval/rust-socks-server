use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr, SocketAddrV6, Ipv6Addr};

use authentication::Method;

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
                    Err("Packet length does not match nmethods.".to_owned())
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

pub enum RequestError {
    CommandNotSupported(u8),
    AddressTypeNotSupported(u8),
    PacketTooShort,
}

macro_rules! read_u16 {
    ( $bytes:expr, $index:expr ) => { 
        (($bytes[$index] as u16) << 8) + ($bytes[$index+1] as u16)
    }
}

pub enum Address {
    /// IPv4 or IPv6 address and a port.
    SocketAddr(SocketAddr),
    /// Domain and port
    DomainPort(Vec<u8>, u16),
}

impl Address {
    pub fn decode(bytes: &[u8]) -> Result<Address, RequestError> {
        match bytes.get(0) {
            Some(&0x01) => { // IPv4
                match (bytes.get(8), bytes.get(9)) {
                    (Some(port1), Some(port2)) => {
                        let addr = Ipv4Addr::new(bytes[4], bytes[5], bytes[6], bytes[7]);
                        let port = ((*port1 as u16) << 8) + (*port2 as u16);
                        Ok(Address::SocketAddr(SocketAddr::V4(SocketAddrV4::new(addr, port))))
                    },
                    _ => Err(RequestError::PacketTooShort),
                }
            },
            Some(&0x04) => { // IPv6
                match (bytes.get(20), bytes.get(21)) {
                    (Some(port1), Some(port2)) => {
                        let addr = Ipv6Addr::new(
                            read_u16!(bytes,  4), read_u16!(bytes,  6),
                            read_u16!(bytes,  8), read_u16!(bytes, 10),
                            read_u16!(bytes, 12), read_u16!(bytes, 14),
                            read_u16!(bytes, 16), read_u16!(bytes, 18),
                            );
                        let port = ((*port1 as u16) << 8) + (*port2 as u16);
                        let flowinfo = 0;
                        let scope_id = 0; // TODO: change this?
                        Ok(Address::SocketAddr(SocketAddr::V6(SocketAddrV6::new(addr, port, flowinfo, scope_id))))
                    },
                    _ => Err(RequestError::PacketTooShort),
                }
            },
            Some(&0x03) => { // Domain
                let length = match bytes.get(4) {
                    Some(length) => *length as usize,
                    None => return Err(RequestError::PacketTooShort),
                };
                match (bytes.get(length+5), bytes.get(length+6)) {
                    (Some(port1), Some(port2)) => {
                        let port = ((*port1 as u16) << 8) + (*port2 as u16);
                        let domain = bytes[5..length+5].to_vec();
                        Ok(Address::DomainPort(domain, port))
                    },
                    _ => Err(RequestError::PacketTooShort),
                }
            },
            Some(address_type) => Err(RequestError::AddressTypeNotSupported(*address_type)),
            None => Err(RequestError::PacketTooShort)
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match *self {
            Address::SocketAddr(SocketAddr::V4(saddr)) => {
                let mut res = vec![0u8; 7];
                res[0] = 0x01;
                res[1..5].copy_from_slice(&saddr.ip().octets());
                res[6] = (saddr.port() >> 8) as u8;
                res[7] = (saddr.port() & 0xff) as u8;
                res
            }
            Address::SocketAddr(SocketAddr::V6(saddr)) => {
                let mut res = vec![0u8; 19];
                res[0] = 0x04;
                res[1..17].copy_from_slice(&saddr.ip().octets());
                res[18] = (saddr.port() >> 8) as u8;
                res[19] = (saddr.port() & 0xff) as u8;
                res
            }
            Address::DomainPort(ref domain, port) => {
                let domain_length = domain.len();
                assert!(domain_length < 256);
                let mut res = vec![0u8; 2+domain_length+2];
                res[0] = 0x03;
                res[1] = domain_length as u8;
                res[2..2+domain_length].copy_from_slice(domain);
                res[2+domain_length] = (port >> 8) as u8;
                res[2+domain_length+1] = (port & 0xff) as u8;
                res
            }
        }
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
