use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::time::Duration;
use libc::ENOENT;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};

use smash_arc as arc;
use smash_arc::{ArcFile, ArcLookup};


struct ArcFS {
    pub arc: ArcFile,
}

#[derive(Debug)]
struct OpenError;

impl ArcFS {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OpenError> {
        Ok(Self { arc: ArcFile::open(path).map_err(|_| OpenError)? })
    }
}

impl Filesystem for ArcFS {
    fn init(&mut self, _req: &Request) -> Result<(), i32> {
        println!("Arc successfully mounted");
        Ok(())
    }

    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let parent = if parent == 1 { 0 } else { parent };
        if let Some(a) = self.arc.get_name(parent) {
            let file_path = String::from(a) +
                if a.len() != 0 { "/" } else { "" } +
                name.to_str().unwrap();
            
            let hash40 = arc::hash40(&file_path);
            match self.arc.files.get(&hash40) {
                Some(arc::FileNode::Dir(hash)) => {
                    reply.entry(&Duration::from_secs(1), &FileAttr {
                            ino: hash40.as_u64(),
                            size: 0,
                            blocks: 0,
                            atime: UNIX_EPOCH,
                            mtime: UNIX_EPOCH,
                            ctime: UNIX_EPOCH,
                            crtime: UNIX_EPOCH,
                            kind: FileType::Directory,
                            perm: 0o755,
                            nlink: 2,
                            uid: req.uid(),
                            gid: req.gid(),
                            rdev: 0,
                            flags: 0, 
                    }, 0);
                }
                Some(arc::FileNode::Uncompressed { data, .. }) => {
                    reply.entry(&Duration::from_secs(1), &FileAttr {
                        ino: hash40.as_u64(),
                        size: data.len() as u64,
                        blocks: 1,
                        atime: UNIX_EPOCH,
                        mtime: UNIX_EPOCH,
                        ctime: UNIX_EPOCH,
                        crtime: UNIX_EPOCH,
                        kind: FileType::RegularFile,
                        perm: 0o644,
                        nlink: 1,
                        uid: req.uid(),
                        gid: req.gid(),
                        rdev: 0,
                        flags: 0, 
                    }, 0);
                }
                Some(arc::FileNode::Compressed { decomp_size, .. }) => {
                    reply.entry(&Duration::from_secs(1), &FileAttr {
                        ino: hash40.as_u64(),
                        size: *decomp_size,
                        blocks: 1,
                        atime: UNIX_EPOCH,
                        mtime: UNIX_EPOCH,
                        ctime: UNIX_EPOCH,
                        crtime: UNIX_EPOCH,
                        kind: FileType::RegularFile,
                        perm: 0o644,
                        nlink: 1,
                        uid: req.uid(),
                        gid: req.gid(),
                        rdev: 0,
                        flags: 0, 
                    }, 0);
                }
                None => {
                    //dbg!("File does not exist", hash40);
                    //dbg!(a, file_path, name);
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

    fn getattr(&mut self, req: &Request, ino: u64, reply: ReplyAttr) {
        let ino = if ino == 1 { 0 } else { ino };
        match self.arc.files.get(&ino) {
            Some(arc::FileNode::Directory) => {
                reply.attr(&Duration::from_secs(1), &FileAttr {
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
                        uid: req.uid(),
                        gid: req.gid(),
                        rdev: 0,
                        flags: 0, 
                });
            }
            Some(arc::FileNode::Uncompressed { data, .. }) => {
                reply.attr(&Duration::from_secs(1), &FileAttr {
                    ino,
                    size: data.len() as u64,
                    blocks: 1,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: req.uid(),
                    gid: req.gid(),
                    rdev: 0,
                    flags: 0, 
                });
            }
            Some(arc::FileNode::Compressed { decomp_size, .. }) => {
                reply.attr(&Duration::from_secs(1), &FileAttr {
                    ino,
                    size: *decomp_size,
                    blocks: 1,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: req.uid(),
                    gid: req.gid(),
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
        match self.arc.get_file_contents(ino) {
            Ok(data) => {
                let start = offset as usize;
                let end = usize::min((offset as usize) + (size as usize), data.len());
                reply.data(&data[start..end]);
            }
            Err(err) => {
                eprintln!("Failed to read data: {:?}", err);
                reply.error(ENOENT);
            }
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let ino = if ino == 1 { 0 } else { ino };
        match self.arc.get_dir_listing(ino) {
            Some(children) => {
                let mut entries = vec![
                    (1, FileType::Directory, "."),
                    (1, FileType::Directory, ".."),
                ];
                let labels = arc::GLOBAL_LABELS.read();
                for child in children {
                    match child {
                        arc::FileNode::Dir(dir) => {
                            let label;
                            entries.push((
                                dir.as_u64(),
                                FileType::Directory,
                                dir.label(&labels).unwrap_or_else(|| {
                                    label = format!("{:#x}", dir.as_u64());
                                    &label
                                })
                            ));
                        }
                        arc::FileNode::File(file) => {
                            let label;
                            entries.push((
                                file.as_u64(),
                                FileType::RegularFile,
                                file.label(&labels).unwrap_or_else(|| {
                                    label = format!("{:#x}", file.as_u64());
                                    &label
                                })
                            ));
                        }
                    }
                }
                
                let to_skip = if offset == 0 { offset } else { offset + 1 } as usize;
                for (i, entry) in entries.into_iter().enumerate().skip(to_skip) {
                    reply.add(entry.0, i as i64, entry.1, entry.2);
                }
                reply.ok();
            }
            None => {
                println!("readdir: directory not found");
                reply.error(ENOENT);
            }
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
    if let Some((arc_path, mountpoint)) = get_args() {
        let options = ["-o", "ro", "-o", "fsname=hello", "-o", "auto_unmount", "-o", "allow_other"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();
        fuse::mount(ArcFS::open(arc_path).unwrap(), &mountpoint, &options).unwrap();
    } else {
        eprintln!("Missing arg [mountpoint]");
    }
}
