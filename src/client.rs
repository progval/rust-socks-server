//! Contains the `Client` structure, which represents a connection
//! to a client.

use authentication::Method;
use command::Command;
use request_error::RequestError;
use address::Address;
use messages;

const PROTOCOL_VERSION: u8 = 5;

/// A new client, whose request has not yet been accepted.
pub struct NewUnauthenticatedClient {
    methods: Vec<Method>,
}

impl NewUnauthenticatedClient {
    /// Processes a packet from a new client.
    ///
    /// Two possible results:
    /// * If there is a protocol error, `Err(error_message)` is returned.
    ///   It may be written on the console or in a log file.
    /// * If everything went well, `Ok(new_unauthenticated_client)` is returned
    ///   and should be used to accept or not the client.
    pub fn new<F>(initial_packet: &[u8]) -> Result<NewUnauthenticatedClient, String> {
        let message = try!(messages::InitialMessage::decode(initial_packet));
        if message.version == PROTOCOL_VERSION {
            Ok(NewUnauthenticatedClient { methods: message.methods })
        }
        else {
            Err(format!("Unsupported version: {}", message.version))
        }
    }

    /// Returns methods proposed by the client.
    pub fn methods(&self) -> &Vec<Method> {
        &self.methods
    }

    /// Accepts one of the client's proposed methods.
    /// Panics if the method is `NoAcceptableMethods` or
    /// `UnknownMethod`.
    pub fn accept_method(self, method: Method) -> (NewAuthenticatedClient, Vec<u8>) {
        match method {
            Method::NoAcceptableMethods | Method::UnknownMethod(_) =>
                panic!("NoAcceptableMethods and UnknownMethod may not be \
                    used to accept a client."),
            _ => {}
        }
        let client = NewAuthenticatedClient {};
        let initial_response = messages::InitialResponse {
            version: PROTOCOL_VERSION,
            method: method
        };
        (client, initial_response.encode())
    }

    /// Used to refuse the client's connection.
    /// Returns a reply that should be sent to the client.
    pub fn refuse(self) -> Vec<u8> {
        let response = messages::InitialResponse {
            version: PROTOCOL_VERSION,
            method: Method::NoAcceptableMethods
        };
        response.encode()
    }
}

pub struct NewAuthenticatedClient {
}

impl NewAuthenticatedClient {
    /// Processes the first packet of a new and authenticated client, and
    /// returns an `EarlyClient`
    pub fn on_request(self, packet: &[u8]) -> Result<EarlyClient, RequestError> {
        let message = try!(messages::Request::decode(packet));
        if message.version != PROTOCOL_VERSION {
            Err(RequestError::UnsupportedVersion(message.version))
        }
        else {
            Ok(EarlyClient { command: message.command, dest_address: message.dest_address })
        }
    }
}


/// A client after it made its initial request.
pub struct EarlyClient {
    command: Command,
    dest_address: Address,
}

impl EarlyClient {
    /// Returns the command used by the client
    pub fn command(&self) -> Command {
        self.command
    }
    /// Returns the destination address requested by the client.
    pub fn dest_address(&self) -> &Address {
        &self.dest_address
    }

    /// Accept and confirm success of the early client's request.
    /// Returns a `Client`, and a reply that should be sent to the client.
    pub fn reply_success(self, bound_address: Address) -> (Client, Vec<u8>) {
        let client = Client {
            command: self.command,
            dest_address: self.dest_address,
            bound_address: bound_address.clone(),
        };
        let reply = messages::Reply {
            version: PROTOCOL_VERSION,
            reply: messages::ReplyType::Succeeded,
            bound_address: bound_address,
        };
        (client, reply.encode())
    }
}

pub struct Client {
    command: Command,
    dest_address: Address,
    bound_address: Address,
}

impl Client {
    pub fn command(&self) -> Command {
        self.command
    }
    pub fn dest_address(&self) -> &Address {
        &self.dest_address
    }
    pub fn bound_address(&self) -> &Address {
        &self.bound_address
    }
}
