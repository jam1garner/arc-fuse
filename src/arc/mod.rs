#![allow(dead_code)]
use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io;
use std::mem;

mod util;
mod structs;
mod parser;
use crc::crc32;
use structs::*;
use memmap::Mmap;
use packed_struct::prelude::*;

static HASH_STRINGS: &'static str = include_str!("hashes.txt");

pub struct Arc {
    file: File,
    map: Mmap,
    decomp_table: Vec<u8>,
    stream_entries: Vec<StreamEntry>,
    stream_file_indices: Vec<u32>,
    stream_offset_entries: Vec<StreamOffsetEntry>,
    names: HashMap<u32, &'static str>,
    file_listing: FileNode,
}

pub enum FileNode {
    UncompressedFile {
        offset: u64,
        size: u64,
        flags: u32,
    },
    Directory {
        nodes: HashMap<&'static str, FileNode>
    },
    None
}

impl<'a> Arc {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let file = File::open(path.as_ref())?;
        let map = unsafe { Mmap::map(&file) }?;

        let mut arc = Arc {
            file,
            map,
            decomp_table: vec![],
            stream_entries: vec![],
            stream_file_indices: vec![],
            stream_offset_entries: vec![],
            names: HashMap::new(),
            file_listing: FileNode::None,
        };

        arc.decompress_table().unwrap();

        let node_header = arc.read_from_table::<NodeHeader>(0);
        let node_header_2 = arc.read_from_table::<NodeHeader2>(0x100);
        
        let mut pos = 0x100 + mem::size_of::<NodeHeader2>();
        macro_rules! read {
            ($t:ty) => (
                {
                    let temp = arc.read_from_table::<t>(pos);
                    pos += mem::size_of::<t>();
                    temp
                }
            );
            ($t:ty, $n:expr) => (
                {
                    let temp = arc.read_from_table_slice::<$t>(pos, $n);
                    pos += mem::size_of::<$t>() * $n;
                    temp
                }
            )
        }
        macro_rules! skip {
            ($t:ty, $n:expr) => (
                pos += mem::size_of::<$t>() * $n;
            );
            ($s:expr, $n:expr) => (
                pos += $s * $n;
            )
        }

        skip!(8, node_header_2.part1_count as usize);
        arc.stream_entries =
            (0..node_header_2.part1_count)
            .map(|_|{
                pos += 0xC;
                StreamEntry::unpack(array_ref!(
                        &arc.decomp_table[pos-0xC..pos],
                0, 0xC)).unwrap()
            }).collect();
        arc.stream_file_indices = Vec::from(read!(u32, node_header_2.part2_count as usize));
        arc.stream_offset_entries = Vec::from(read!(StreamOffsetEntry, node_header_2.part3_count as usize));
        
        arc.load_hashes();
        arc.load_stream_files();

        Ok(arc)
    }

    fn read_from_offset<T>(&self, offset: usize) -> &'a T {
        unsafe {
            mem::transmute(self.map.as_ptr().offset(offset as isize))
        }
    }

    fn read_from_table<T>(&self, offset: usize) -> &'a T {
        unsafe {
            mem::transmute(self.decomp_table.as_ptr().offset(offset as isize))
        }
    }
    
    fn read_from_table_slice<T>(&self, offset: usize, count: usize) -> &'a [T] {
        unsafe {
            std::slice::from_raw_parts(
                mem::transmute(self.decomp_table.as_ptr().offset(offset as isize))
            , count)
        }
    }

    fn header(&self) -> &'a ArcHeader {
        self.read_from_offset(0)
    }

    fn compressed_table(&self) -> &'a [u8] {
        let node_offset = self.header().node_section_offset as usize;
        unsafe {
            let comp_table_header = self.read_from_offset::<CompTableHeader>(node_offset);
            const S: usize = mem::size_of::<CompTableHeader>();
            mem::transmute(
                &self.map[node_offset + S..node_offset + S + comp_table_header.comp_size as usize]
            )
        }
    }

    fn decompress_table(&mut self) -> io::Result<()> {
        let compressed_table = io::Cursor::new(self.compressed_table());
        self.decomp_table = zstd::stream::decode_all(compressed_table)?;
        Ok(())
    }
    
    fn load_hashes(&mut self) {
        let lines: Vec<&'static str> = HASH_STRINGS.split('\n').collect();
        for line in lines {
            self.names.insert(
                crc32::checksum_ieee(line.as_bytes()),
                line
            );
        }
    }

    fn make_dir_recursive() -> FileNode {
        FileNode::None
    }

    fn load_stream_files(&mut self) {
        if let FileNode::None = self.file_listing {
            self.file_listing = FileNode::Directory {
                nodes: HashMap::new()
            };
        }
        for stream_file in &self.stream_entries {
            if let Some(path) = self.names.get(&stream_file.hash) {
                let path_components: Vec<&'static str> = path.split("/").collect();
                let mut pos = 0;
                for dir in &path_components[..path_components.len()-1] {
                    
                    pos += 1 + dir.len();
                }
            }
        }
    }
}

