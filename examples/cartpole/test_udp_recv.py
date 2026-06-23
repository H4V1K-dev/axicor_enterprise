import socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind(("127.0.0.1", 8081))
print("Listening on 8081...")
while True:
    data, addr = sock.recvfrom(1024)
    print(f"Received {len(data)} bytes from {addr}")
