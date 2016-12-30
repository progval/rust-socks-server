//! Contains the `Client` structure, which represents a connection
//! to a client.

use authentication::Method;
use messages::*;

enum ClientState {
    NoAcceptableAuthenticationMethod,
    Authenticated,
}

/// Represents a connection to a client.
pub struct Client {
    state: ClientState,
}

const PROTOCOL_VERSION: u8 = 5;

impl Client {
    pub fn new<F>(initial_packet: &[u8], method_selector: F) -> Result<(Client, Vec<u8>), String>
            where F: Fn(&[Method]) -> Method {
        let message = try!(InitialMessage::decode(initial_packet));
        if message.version != PROTOCOL_VERSION {
            return Err(format!("Unsupported version: {}", message.version));
        }
        let method = method_selector(&message.methods);
        
        match method {
            Method::NoAuthenticationRequired => {
                let client = Client {
                    state: ClientState::Authenticated
                };
                let initial_response = InitialResponse {
                    version: PROTOCOL_VERSION,
                    method: method
                };
                Ok((client, initial_response.encode()))
            },
            Method::UnknownMethod(_) | Method::NoAcceptableMethods => {
                let client = Client {
                    state: ClientState::NoAcceptableAuthenticationMethod
                };
                let initial_response = InitialResponse {
                    version: PROTOCOL_VERSION,
                    method: Method::NoAcceptableMethods
                };
                Ok((client, initial_response.encode()))
            }
        }
    }
}
