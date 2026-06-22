# /// script
# requires-python = ">=3.8"
# dependencies = []
# ///

import socket
import time

HOST = '127.0.0.1'
PORT = 2323

def run_server():
    # Create a raw IPv4 TCP socket
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        # Prevent "Address already in use" errors if you restart the script quickly
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        s.bind((HOST, PORT))
        s.listen()

        print(f"[*] Dummy Telnet Server listening on {HOST}:{PORT}")
        print("[*] Waiting for your Rust client to connect...")

        conn, addr = s.accept()
        with conn:
            print(f"\n[+] Connection established from {addr}")

            # --- THE FRAGMENTATION TEST ---
            print(">>> Sending partial sequence: [255] (IAC)")
            conn.sendall(b"\xff")

            # This 1.5 second delay forces your Rust `tokio::select!` loop to fire,
            # read the first byte, and spin back around.
            time.sleep(1.5)

            print(">>> Sending remainder: [253, 1] (DO ECHO)")
            conn.sendall(b"\xfd\x01")

            time.sleep(0.5)
            print(">>> Sending normal text payload")
            conn.sendall(b"\r\nWelcome to the test server. Type something:\r\n")

            # --- THE ECHO LOOP ---
            # This allows you to test the `stdin` branch of your Rust client
            while True:
                data = conn.recv(1024)
                if not data:
                    print("\n[-] Client disconnected.")
                    break

                print(f"RCVD: {data}")
                # Echo it back so it prints on your Rust client's stdout
                conn.sendall(b"Server Echo: " + data)

if __name__ == "__main__":
    run_server()