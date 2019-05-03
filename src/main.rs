extern crate fuse;
extern crate zstd;
extern crate memmap;
extern crate env_logger;
extern crate packed_struct;
#[macro_use] extern crate arrayref;
#[macro_use] extern crate packed_struct_codegen;

use std::env;
use std::ffi::OsStr;
use std::path::Path;
use time::Timespec;
use libc::ENOENT;
use std::collections::HashMap;
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

struct ArcFS {
    pub arc: arc::Arc,
}

impl ArcFS {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(Self {
            arc: arc::Arc::open(path)?,
        })
    }
}

impl Filesystem for ArcFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let parent = if parent == 1 { 0 } else { parent };
        if let Some(a) = self.arc.get_name(parent) {
            let file_path = String::from(a.clone()) +
                if a.len() != 0 { "/" } else { "" } +
                name.to_str().unwrap();
            
            let hash40 = arc::Arc::hash40(&file_path);
            match self.arc.files.get(&hash40) {
                Some(arc::ArcFileInfo::Directory) => {
                    reply.entry(&TTL, &FileAttr {
                            ino: hash40,
                            size: 0,
                            blocks: 0,
                            atime: UNIX_EPOCH,
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
                    }, 0);
                }
                Some(arc::ArcFileInfo::Uncompressed {
                    offset: _, size, flags: _
                }) => {
                    reply.entry(&TTL, &FileAttr {
                        ino: hash40,
                        size: *size,
                        blocks: 1,
                        atime: UNIX_EPOCH,
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
                    }, 0);
                }
                None => {
                    dbg!("File does not exist");
                    reply.error(ENOENT);
                }
                _ => {
                    dbg!("Error: filetype match fail");
                    reply.error(ENOENT);
                }
            }
        } else {
            dbg!("Error: name not found");
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match self.arc.files.get(&ino) {
            Some(arc::ArcFileInfo::Directory) => {
                reply.attr(&TTL, &FileAttr {
                        ino,
                        size: 0,
                        blocks: 0,
                        atime: UNIX_EPOCH,
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
                });
            }
            Some(arc::ArcFileInfo::Uncompressed {
                offset: _, size, flags: _
            }) => {
                reply.attr(&TTL, &FileAttr {
                    ino,
                    size: *size,
                    blocks: 1,
                    atime: UNIX_EPOCH,
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
                });
            }
            None => {
                dbg!("File does not exist");
                reply.error(ENOENT);
            }
            _ => {
                dbg!("Error: filetype match fail");
                reply.error(ENOENT);
            }
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        if let Some(data) = self.arc.get_file_data(ino) {
            reply.data(&data[offset as usize..(offset + size as i64) as usize]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let ino = if ino == 1 { 0 } else { ino };
        if let Some(children) = self.arc.dir_children.get(&ino) {
            let mut entries = vec![
                (1, FileType::Directory, "."),
                (1, FileType::Directory, ".."),
            ];
            for child in children {
                if *child == 0 {
                    continue;
                }
                entries.push(
                    (
                        *child,
                        match self.arc.files.get(&child) {
                            Some(arc::ArcFileInfo::Directory) => {
                                FileType::Directory
                            }
                            Some(arc::ArcFileInfo::Uncompressed {
                                offset: _, flags: _, size: _
                            }) => {
                                FileType::RegularFile
                            }
                            _ => {
                                panic!("Improper type")
                            }
                        },
                        self.arc.stems.get(&child).unwrap()
                    )
                )
            }
            
            let to_skip = if offset == 0 { offset } else { offset + 1 } as usize;
            for (i, entry) in entries.into_iter().enumerate().skip(to_skip) {
                println!("{}, {}, {:?}, {}", entry.0, i as i64, entry.1, entry.2);
                reply.add(entry.0, i as i64, entry.1, entry.2);
            }
            reply.ok();
        } else {
            println!("Not found");
            reply.error(ENOENT);
        }
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

    if let Some((arc_path, mountpoint)) = get_args() {
        let options = ["-o", "ro", "-o", "fsname=hello"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();
        fuse::mount(ArcFS::open(arc_path).unwrap(), &mountpoint, &options).unwrap();
    } else {
        eprintln!("Missing arg [mountpoint]");
    }
}
