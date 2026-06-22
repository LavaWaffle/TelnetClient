use bytes::{Buf, BytesMut};
use std::error::Error;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

    loop {
        tokio::select! {
            result = socket_read.read_buf(&mut network_buffer) => {
                let bytes_read = result?;

                if bytes_read == 0 {
                    println!("Connection closed by server");
                    break;
                }

                // todo!(); // Implement telnet read

                println!("Received {bytes_read} bytes: {:?}", &network_buffer[..]);
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
