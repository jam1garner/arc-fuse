from binascii import crc32

with open('./hashes.txt', 'r') as f:
    lines = f.read().split('\n')

def hash40(string):
    return crc32(string.encode('utf8')) + (len(string) << 32)

with open('./hash40s.tsv', 'w') as f:
    for line in lines:
        line_hash = hash40(line)
        print(f"{line_hash:X}\t{line}", file=f)
