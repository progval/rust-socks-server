use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr, SocketAddrV6, Ipv6Addr};

use request_error::RequestError;

macro_rules! read_u16 {
    ( $bytes:expr, $index:expr ) => { 
        (($bytes[$index] as u16) << 8) + ($bytes[$index+1] as u16)
    }
}


#[derive(Clone, Eq, PartialEq, Debug)]
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
                match (bytes.get(5), bytes.get(6)) {
                    (Some(port1), Some(port2)) => {
                        let addr = Ipv4Addr::new(bytes[1], bytes[2], bytes[3], bytes[4]);
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
                let length = match bytes.get(1) {
                    Some(length) => *length as usize,
                    None => return Err(RequestError::PacketTooShort),
                };
                match (bytes.get(length+2), bytes.get(length+3)) {
                    (Some(port1), Some(port2)) => {
                        let port = ((*port1 as u16) << 8) + (*port2 as u16);
                        let domain = bytes[2..length+2].to_vec();
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
                res[5] = (saddr.port() >> 8) as u8;
                res[6] = (saddr.port() & 0xff) as u8;
                res
            }
            Address::SocketAddr(SocketAddr::V6(saddr)) => {
                let mut res = vec![0u8; 19];
                res[0] = 0x04;
                res[1..17].copy_from_slice(&saddr.ip().octets());
                res[17] = (saddr.port() >> 8) as u8;
                res[18] = (saddr.port() & 0xff) as u8;
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
