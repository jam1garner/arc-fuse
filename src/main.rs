use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::time::Duration;
use libc::ENOENT;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};

use smash_arc as arc;
use smash_arc::{ArcFile, ArcLookup, Hash40};


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

        let labels = arc::GLOBAL_LABELS.read();
        if let Some(parent_str) = Hash40(parent).label(&labels) {
            let mut file_path = format!("{}/{}", parent_str.trim_end_matches('/'), name.to_str().unwrap());
            
            let hash40 = arc::hash40(file_path.trim_start_matches('/'));
            match self.arc.get_file_metadata(hash40)  {
                Ok(file_metadata) => {
                    reply.entry(&Duration::from_secs(1), &FileAttr {
                        ino: hash40.as_u64(),
                        size: file_metadata.decomp_size,
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
                Err(arc::LookupError::Missing) => {
                    // No file exists, look for directory
                    if self.arc.get_dir_listing(hash40).is_some() {
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
                    } else {
                        file_path.push('/');
                        let hash40 = arc::hash40(file_path.trim_start_matches('/'));
                        // Try dir with a trailing "/"
                        if self.arc.get_dir_listing(hash40).is_some() {
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
                        } else {
                            // Does not exist
                            reply.error(ENOENT);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("File lookup error: {:?}", err);
                }
            }
        }
    }

    fn getattr(&mut self, req: &Request, ino: u64, reply: ReplyAttr) {
        let hash40 = if ino == 1 { arc::hash40("/") } else { Hash40(ino) };
        match self.arc.get_file_metadata(hash40)  {
            Ok(file_metadata) => {
                reply.attr(&Duration::from_secs(1), &FileAttr {
                    ino: hash40.as_u64(),
                    size: file_metadata.decomp_size,
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
            Err(arc::LookupError::Missing) => {
                // No file exists, look for directory
                if self.arc.get_dir_listing(hash40).is_some() {
                    reply.attr(&Duration::from_secs(1), &FileAttr {
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
                    });
                } else {
                    // Does not exist
                    reply.error(ENOENT);
                }
            }
            Err(err) => {
                eprintln!("File lookup error: {:?}", err);
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
        let ino = if ino == 1 { arc::hash40("/").as_u64() } else { ino };
        match self.arc.get_dir_listing(ino) {
            Some(children) => {
                let mut entries = vec![
                    (1, FileType::Directory, ".".to_owned()),
                    (1, FileType::Directory, "..".to_owned()),
                ];
                let labels = arc::GLOBAL_LABELS.read();
                for child in children {
                    match child {
                        arc::FileNode::Dir(dir) => {
                            entries.push((
                                dir.as_u64(),
                                FileType::Directory,
                                dir.label(&labels)
                                    .map(|s| s.split('/').last().unwrap().to_owned())
                                    .unwrap_or_else(|| format!("{:#x}", dir.as_u64()))
                            ));
                        }
                        arc::FileNode::File(file) => {
                            entries.push((
                                file.as_u64(),
                                FileType::RegularFile,
                                file.label(&labels)
                                    .map(|s| s.split('/').last().unwrap().to_owned())
                                    .unwrap_or_else(|| format!("{:#x}", file.as_u64()))
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

fn get_args() -> Option<(std::ffi::OsString, std::ffi::OsString, std::ffi::OsString)> {
    Some((
        env::args_os().nth(1)?,
        env::args_os().nth(2)?,
        env::args_os().nth(3)?
    ))
}

fn main() {
    if let Some((arc_path, labels, mountpoint)) = get_args() {
        let options = ["-o", "ro", "-o", "fsname=hello", "-o", "auto_unmount", "-o", "allow_other"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();

        Hash40::set_global_labels_file(labels);
        fuse::mount(ArcFS::open(arc_path).unwrap(), &mountpoint, &options).unwrap();
    } else {
        eprintln!("Missing args, format:\narc-fuse [arc path] [labels file] [mount path]");
    }
}
