from sys import argv
from zlib import crc32

crcs = {}
with open('hashes.txt', 'r') as f:
    for i in f.readlines():
        text = i.rstrip()
        crcs[crc32(text.encode('ascii')) & 0xFFFFFFFF] = text

while True:
    try:
        print(crcs[int(input("hash: "), 16) & 0xFFFFFFFF])
    except Exception(e):
        print(e)
