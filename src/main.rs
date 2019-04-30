extern crate fuse;
extern crate zstd;
extern crate memmap;
extern crate env_logger;

use std::env;
use std::ffi::OsStr;
use time::Timespec;
use libc::ENOENT;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};

mod arc;

const TTL: Timespec = Timespec {
    sec: 1,
    nsec: 0,
};

const UNIX_EPOCH: Timespec = Timespec {
    sec: 0,
    nsec: 0,
};

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,                                  // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

const TEST_DIR_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,                                  // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 3,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH,                                  // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

struct HelloFS;

impl Filesystem for HelloFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 2 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        }
        else if parent == 1 && name.to_str() == Some("test") {
            reply.entry(&TTL, &TEST_DIR_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &TEST_DIR_ATTR),
            3 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, _size: u32, reply: ReplyData) {
        if ino == 3 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let entries = match ino {
            1 => {
                vec![
                    (1, FileType::Directory, "."),
                    (1, FileType::Directory, ".."),
                    (2, FileType::Directory, "test"),
                ]
            }
            2 => {
                vec![
                    (1, FileType::Directory, "."),
                    (1, FileType::Directory, ".."),
                    (3, FileType::RegularFile, "hello.txt"),
                ]
            }
            _ => {
                reply.error(ENOENT);
                return;
            }
        };

        // Offset of 0 means no offset.
        // Non-zero offset means the passed offset has already been seen, and we should start after
        // it.
        let to_skip = if offset == 0 { offset } else { offset + 1 } as usize;
        for (i, entry) in entries.into_iter().enumerate().skip(to_skip) {
            reply.add(entry.0, i as i64, entry.1, entry.2);
        }
        reply.ok();
    }
}

fn get_args() -> Option<(std::ffi::OsString, std::ffi::OsString)> {
    Some((
        env::args_os().nth(1)?,
        env::args_os().nth(2)?
    ))
}

fn main() {
    env_logger::init();
    arc::Arc::open("/home/jam/e/Ultimate/300/data.arc");    

    if let Some((arc_path, mountpoint)) = get_args() {
        let options = ["-o", "ro", "-o", "fsname=hello"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();
        fuse::mount(HelloFS, &mountpoint, &options).unwrap();
    } else {
        eprintln!("Missing arg [mountpoint]");
    }
}
