extern crate socks_server;

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::io::ErrorKind;
use std::sync::{Arc, Condvar, Mutex};
use std::io::copy;
use std::thread;

use socks_server::authentication::Method;
use socks_server::client::{NewUnauthenticatedClient, EarlyClient};
use socks_server::command::Command;
use socks_server::messages::ReplyType;
use socks_server::address::Address;

fn pipe_stream(stop: Arc<Condvar>, mut from: TcpStream, mut to: TcpStream) {
    /*
    let mut buf = [0u8; 4096];
    while !*stop.lock().unwrap() {
        match from.read(&mut buf) {
            Ok(_size) => {},
            Err(_) => break,
        }
        match to.write(buf) {
            Ok(_size) => {},
            Err(_) => break,
        }
    }*/
    match copy(&mut from, &mut to) {
        Ok(_size) => {},
        Err(_err) => {},
    }
    stop.notify_all()
}

fn handle_connect(early_client: EarlyClient, mut client_stream: TcpStream) {
    // Open TCP stream to the destination
    let server_stream_result = match *early_client.dest_address() {
        Address::DomainPort(ref domain, port) => {
            let domain = String::from_utf8(domain.clone()).unwrap();
            println!("Connecting to: {}:{}.", domain, port);
            TcpStream::connect((&*domain, port))
        },
        Address::SocketAddr(ref saddr) => {
            println!("Connecting to: {:?}", saddr);
            TcpStream::connect(saddr)
        },
    };

    // If there is an error, tell the client about it and close the client's
    // stream.
    let server_stream = match server_stream_result {
        Ok(server_stream) => server_stream,
        Err(err) => {
            println!("Error: {:?}", err);
            let reply_type = match err.kind() {
                ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset |
                ErrorKind::ConnectionAborted =>
                    ReplyType::ConnectionRefused,
                ErrorKind::AddrNotAvailable =>
                    ReplyType::NetworkUnreachable,
                _ => ReplyType::GeneralFailure,
            };
            let reply = early_client.reply_error(reply_type);
            client_stream.write(&reply).unwrap();
            return;
        }
    };

    // Tell the client the connection succeeded
    let saddr = Address::SocketAddr(server_stream.local_addr().unwrap());
    let (_client, reply) = early_client.reply_success(saddr);
    client_stream.write(&reply).unwrap();

    let stop = Arc::new(Condvar::new());

    let stop1 = stop.clone();
    let client_stream1 = client_stream.try_clone().unwrap();
    let client_stream2 = client_stream;
    let stop2 = stop.clone();
    let server_stream1 = server_stream.try_clone().unwrap();
    let server_stream2 = server_stream;

    let _client_to_server_thread = thread::spawn(move ||
        pipe_stream(stop1, client_stream1, server_stream1)
    );
    let _server_to_client_thread = thread::spawn(move ||
        pipe_stream(stop2, server_stream2, client_stream2)
    );
    let foo = Mutex::new(0);
    let bar = foo.lock().unwrap();
    stop.wait(bar).unwrap();
}

fn handle_stream(mut stream: TcpStream) {
    // Exchange initial messages
    let max_expected_bytes = NewUnauthenticatedClient::max_expected_bytes();
    let mut packet = vec![0u8; max_expected_bytes];
    let size = stream.read(&mut packet).unwrap();
    packet.resize(size, 0);
    let client = NewUnauthenticatedClient::new(&packet).unwrap();
    if !client.methods().contains(&Method::NoAuthenticationRequired) {
        let reply = client.refuse();
        println!("Error: No supported authentication method.");
        stream.write(&reply).unwrap();
        return;
    }
    let (client, reply) = client.accept_method(Method::NoAuthenticationRequired);
    stream.write(&reply).unwrap();

    // Read request
    let max_expected_bytes = client.max_expected_bytes();
    let mut request = vec![0u8; max_expected_bytes];
    let size = stream.read(&mut request).unwrap();
    packet.resize(size, 0);
    let client = client.on_request(&request).unwrap();
    match client.command() {
        Command::Connect => handle_connect(client, stream),
        Command::Bind | Command::UdpAssociate => {
            let reply = client.reply_error(ReplyType::CommandNotSupported);
            stream.write(&reply).unwrap();
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:1080").unwrap();
    for stream in listener.incoming() {
        handle_stream(stream.unwrap());
    }
}
