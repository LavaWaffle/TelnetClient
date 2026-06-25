use bytes::{Buf, BytesMut};
use std::error::Error;
use std::io::Read;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::io::Write;

pub mod TelnetConsts {
    pub const IAC: u8 = 0xFF;
    pub const DONT: u8 = 0xFE;
    pub const DO: u8 = 0xFD;
    pub const WONT: u8 = 0xFC;
    pub const WILL: u8 = 0xFB;

    pub const ECHO: u8 = 0x01;
}

#[derive(Clone, Copy, PartialEq)]
pub enum NegState {
    No,       // Option is disabled
    Yes,      // Option is enabled
    WantNo,   // We asked to disable, waiting for reply
    WantYes,  // We asked to enable, waiting for reply
}

impl Default for NegState {
    fn default() -> Self {
        NegState::No
    }
}

#[derive(Default, Clone, Copy)]
pub struct OptionState {
    pub local: NegState,  // Our state (WILL/WONT)
    pub remote: NegState, // Their state (DO/DONT)
}

pub struct TelnetClient {
    // Option = Idx
    options: [OptionState; 256],
}

impl TelnetClient {
    pub fn new() -> Self {
        Self {
            options: [OptionState::default(); 256],
        }
    }

    pub fn set_remote(&mut self, idx: u8, state: NegState) {
        self.options[idx as usize].remote = state;
    }

    pub fn set_local(&mut self, idx: u8, state: NegState) {
        self.options[idx as usize].local = state;
    }

    pub fn is_remote(&self, idx: u8, state: NegState) -> bool {
        self.options[idx as usize].remote == state
    }
    pub fn is_local(&self, idx: u8, state: NegState) -> bool {
        self.options[idx as usize].local == state
    }
}
enum TelnetState {
    AwaitingIAC,
    AwaitingCmd,
    AwaitingOpt(u8),
}

#[derive(Debug)]
pub enum TelnetEvent {
    Text(Vec<u8>),
    Command(u8, u8),     // e.g., (DO, ECHO)
    Subnegotiation(Vec<u8>),
}

pub struct TelnetParser {
    state: TelnetState,
    text_buffer: Vec<u8>,
}

impl TelnetParser {
    pub fn new() -> Self {
        Self {
            state: TelnetState::AwaitingIAC,
            text_buffer: Vec::new(),
        }
    }

    #[inline]
    pub fn push_text_buffer(&mut self, events: &mut Vec<TelnetEvent>) {
        if !self.text_buffer.is_empty() {
            let text = std::mem::take(&mut self.text_buffer);
            events.push(TelnetEvent::Text(text));
        }
    }

    pub fn parse_bytes(&mut self, data: &[u8]) -> Vec<TelnetEvent> {
        let mut events: Vec<TelnetEvent> = Vec::new();

        for &byte in data {
            match self.state {
                TelnetState::AwaitingIAC => {
                    if byte == TelnetConsts::IAC {
                        self.state = TelnetState::AwaitingCmd;
                    } else {
                        self.text_buffer.push(byte);
                    }
                }

                TelnetState::AwaitingCmd => {
                    match byte {
                        TelnetConsts::DO | TelnetConsts::DONT |
                        TelnetConsts::WILL | TelnetConsts::WONT => {
                            self.push_text_buffer(&mut events);
                            self.state = TelnetState::AwaitingOpt(byte);
                        }
                        TelnetConsts::IAC => {
                            // 0xff 0xff -> 0xff into char stream
                            self.text_buffer.push(byte);
                            self.state = TelnetState::AwaitingIAC;
                        }
                        _ => {
                            println!("[TelnetParser] Did not receive IAC or CMD after receiving IAC");
                            self.push_text_buffer(&mut events);
                            self.state = TelnetState::AwaitingIAC;
                        }
                    }
                }
                TelnetState::AwaitingOpt(cmd) => {
                    events.push(
                        TelnetEvent::Command(cmd, byte)
                    );

                    self.state = TelnetState::AwaitingIAC;
                }
            }
        }

        // push any remaining text buffer
        self.push_text_buffer(&mut events);

        events
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_addr = "127.0.0.1:2323";
    let mut stream = TcpStream::connect(server_addr)
        .await
        .map_err(|e| format!("Failed to connect to {server_addr}, check 'unit_test.py'. Err: {e}"))?;
    println!("Connected to {server_addr}");

    let (mut socket_read, mut socket_write) = stream.split();

    let mut network_buffer = BytesMut::with_capacity(4096);

    let mut stdin = io::stdin();
    let mut stdin_buffer = [0u8; 1024];

    let mut state = TelnetState::AwaitingIAC;
    let mut tel_cli = TelnetClient::new();
    let mut tel_parser = TelnetParser::new();

    loop {
        tokio::select! {
            result = socket_read.read_buf(&mut network_buffer) => {
                let bytes_read = result?;

                if bytes_read == 0 {
                    println!("Connection closed by server");
                    break;
                }
                println!("Received {bytes_read} bytes: {:02X?}", &network_buffer[..]);
                let events = tel_parser.parse_bytes(&network_buffer[..bytes_read]);
                println!("Formatted into {events:?}");
                
                for event in events {
                    match event {
                        TelnetEvent::Text(bytes) => {
                            match std::str::from_utf8(&bytes) {
                                Ok(valid_str) => {
                                    print!("{}", valid_str);
                                    std::io::stdout().flush().unwrap();
                                },
                                Err(e) => println!("Invalid UTF-8 sequence: {}", e),
                            }
                        }
                        TelnetEvent::Command(cmd, opt) => {
                            if cmd == TelnetConsts::DO {
                                tel_cli.set_remote(opt, NegState::WantYes);

                                if opt == TelnetConsts::ECHO && !tel_cli.is_local(TelnetConsts::ECHO, NegState::Yes){
                                    tel_cli.set_local(TelnetConsts::ECHO, NegState::Yes);
                                    socket_write.write_all(&[TelnetConsts::IAC, TelnetConsts::WILL, opt]).await?;
                                }
                            } else if cmd == TelnetConsts::DONT {
                                tel_cli.set_remote(opt, NegState::WantNo);
                            } else if cmd == TelnetConsts::WILL {
                                tel_cli.set_remote(opt, NegState::Yes);
                            } else if cmd == TelnetConsts::WONT {
                                tel_cli.set_remote(opt, NegState::No);
                            }
                        }
                        TelnetEvent::Subnegotiation(_) => {
                            todo!();
                        }
                    }
                }

                network_buffer.advance(bytes_read);
            }

            result = stdin.read(&mut stdin_buffer) => {
                let bytes_read = result?;

                if bytes_read == 0 {
                    break;
                }

                socket_write.write_all(&stdin_buffer[..bytes_read]).await?;
            }
        }
    }

    Ok(())
}
