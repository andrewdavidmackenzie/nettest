use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::io::{Result, Write, Read};
use time;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

const BUFFER_SIZE: usize = 1024 * 1024;

pub struct TestServer {
    listener: TcpListener,
    listen_address: String,
    listen_port: u16
}

impl TestServer {

    pub fn new(port: u16, address: &str) -> TestServer {
        TestServer {
            listener: TcpListener::bind((address as &str, port)).unwrap(),
            listen_address: address.to_string(),
            listen_port: port
        }
    }

    pub fn listen(self) {
        println!("Listening on host: {} port {}", self.listen_address, self.listen_port);

        for stream in self.listener.incoming() {
            self.new_connection(stream.unwrap());
        }
    }

    fn new_connection(&self, stream: TcpStream) {
        match stream.peer_addr() {
            Ok(addr) => {
                println!("Incoming connection from {}", addr);
                let addr_ = addr.clone();
                thread::spawn(move || {
                    let mut con = Connection::new(stream);
                    match con.handle() {
                        Ok(_) => println!("Connection from {} closed", addr_),
                        Err(x) => println!("Error while reading from connection from {}: {}", addr_, x)
                    };
                });
            },
            Err(x) => {
                println!("Could not retrieve peer address: {}", x)
            }
        }
    }
}

struct Connection {
    stream: TcpStream,
    sender_commander: Sender<u64>
}

impl Connection {

    fn new<'b>(stream: TcpStream) -> Connection {
        let (tx, rx) = channel::<u64>();
        let s = stream.try_clone().unwrap();
        thread::spawn(|| {
            let peer_addr =  s.peer_addr().unwrap().clone();
            match Connection::sender_runner(rx, s) {
                Ok(_) => {}
                Err(x) => println!("Error while writing to connection from {}: {}", peer_addr, x)
            };
        });
        Connection {
            stream: stream,
            sender_commander: tx
        }
    }

    pub fn handle(&mut self) -> Result<()> {
        loop {
            let cmd = try!(self.stream.read_u8());
            match cmd {
                0 => {
                    let mut sink = [0; BUFFER_SIZE];
                    try!(self.stream.read_exact(&mut sink));
                },
                1 => { // Request for Payload
                    let ms = try!(self.stream.read_u64::<BigEndian>());
                    self.sender_commander.send(ms).unwrap();
                },
                3 => { // Pingtest
                    try!(self.stream.write_u8(3u8));
                },
                255 => { // Disconnect
                    return Ok(());
                },
                _ => {
                    println!("Unexpected command {}", cmd);
                }
            };
        }
    }

    fn sender_runner(rx: Receiver<u64>, mut stream: TcpStream) -> Result<()> {

        let buf = [0u8; BUFFER_SIZE];

        loop {
            match rx.recv() {
                Ok(time) => {
                    let start = time::precise_time_ns();
                    loop {

                        try!(stream.write_u8(0u8));
                        try!(stream.write(&buf));
                        try!(stream.flush());

                        if (time::precise_time_ns() - start) / 1_000_000 >= time {
                            break;
                        }
                    }
                    try!(stream.write_u8(2))
                },
                Err(_) => return Ok(())
            }
        }
    }
}
