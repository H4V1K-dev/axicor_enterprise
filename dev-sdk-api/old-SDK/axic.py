import struct

class AxicReader:
    """Zero-Copy Python VFS Reader for .axic archives"""
    def __init__(self, path: str):
        self.path = path
        with open(path, "rb") as f:
            header = f.read(12)
            if header[:4] != b"AXIC": raise ValueError("Invalid AXIC magic")
            # header[8:12] is the count
            self.count = struct.unpack("<I", header[8:12])[0]
            self.toc = {}
            for _ in range(self.count):
                entry = f.read(272)
                name_end = entry.find(b'\x00')
                if name_end == -1: name_end = 256
                name = entry[:name_end].decode('utf-8')
                offset, size = struct.unpack("<QQ", entry[256:272])
                self.toc[name] = (offset, size)

    def read_file(self, internal_path: str) -> bytes:
        if internal_path not in self.toc: return None
        off, size = self.toc[internal_path]
        with open(self.path, "rb") as f:
            f.seek(off)
            return f.read(size)
