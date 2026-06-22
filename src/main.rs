use bytes::{Buf, BytesMut};
use std::error::Error;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod TelnetConsts {
    pub const IAC: u8 = 0xFF;
    pub const DONT: u8 = 0xFE;
    pub const DO: u8 = 0xFD;
    pub const WONT: u8 = 0xFC;
    pub const WILL: u8 = 0xFB;

    pub const ECHO: u8 = 0x01;
}

enum TelnetState {
    NormalText,
    AwaitingCmd,
    AwaitingOpt,
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

    let mut state = TelnetState::NormalText;

    loop {
        tokio::select! {
            result = socket_read.read_buf(&mut network_buffer) => {
                let bytes_read = result?;

                if bytes_read == 0 {
                    println!("Connection closed by server");
                    break;
                }

                for (idx, byte) in network_buffer.iter().enumerate() {
                    if idx >= bytes_read {
                        break;
                    }

                    match state {
                        TelnetState::NormalText => {
                            match *byte {
                                TelnetConsts::IAC => {
                                    state = TelnetState::AwaitingCmd;
                                    println!("Recv 0xFF moving to Await Cmd State");
                                }
                                _ => {
                                    print!("{}", *byte as char);
                                }
                            }
                        }
                        TelnetState::AwaitingCmd => {
                            match *byte {
                                TelnetConsts::DO => {
                                    state = TelnetState::AwaitingOpt;
                                    println!("Recv Cmd: 0xFD (DO) moving to Await Opt State");
                                }
                                _ => {
                                    state = TelnetState::NormalText;
                                    println!("Unrecognized command byte: {:02X}, returning to NormalText", byte);
                                    state = TelnetState::NormalText;
                                }
                            }
                        }
                        TelnetState::AwaitingOpt => {
                            if *byte == TelnetConsts::ECHO {
                                state = TelnetState::NormalText;
                                println!("Recv Opt: 0x01 (ECHO) moving to Normal Text State");
                                socket_write.write_all(&[TelnetConsts::IAC, TelnetConsts::WILL, TelnetConsts::ECHO]).await?;
                            }
                        }
                    }
                }

                println!("Received {bytes_read} bytes: {:02X?}", &network_buffer[..]);
                network_buffer.clear();
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
