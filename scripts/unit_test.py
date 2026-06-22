# /// script
# requires-python = ">=3.8"
# dependencies = []
# ///

import socket
import time

HOST = '127.0.0.1'
PORT = 2323

def run_server():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
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
            time.sleep(1.5)

            print(">>> Sending remainder: [253, 1] (DO ECHO)")
            conn.sendall(b"\xfd\x01")

            # --- THE 10-SECOND WAIT BLOCK ---
            print("[*] Waiting up to 10 seconds for client to reply with WILL/WONT ECHO...")
            conn.settimeout(10.0) # Set the 10-second timer

            try:
                response = conn.recv(1024)
                print(f"RCVD NEGOTIATION: {response}")

                # Check if the response contains WILL ECHO (\xff\xfb\x01) or WONT ECHO (\xff\xfc\x01)
                if b"\xff\xfb\x01" in response or b"\xff\xfc\x01" in response:
                    print("[+] TEST PASSED: Client replied with valid Telnet negotiation!")
                else:
                    print("[-] TEST FAILED: Received data, but not the expected WILL/WONT ECHO.")
                    return # Exit the test

            except TimeoutError:
                print("[-] TEST FAILED: Client took longer than 10 seconds to reply.")
                return # Exit the test

            # Reset the socket back to normal blocking mode so the rest of the script works
            conn.settimeout(None)
            # --------------------------------

            time.sleep(0.5)
            print(">>> Sending normal text payload")
            conn.sendall(b"\r\nWelcome to the test server. Type something:\r\n")

            # --- THE ECHO LOOP ---
            while True:
                data = conn.recv(1024)
                if not data:
                    print("\n[-] Client disconnected.")
                    break

                print(f"RCVD: {data}")
                conn.sendall(b"Server Echo: " + data)

if __name__ == "__main__":
    run_server()