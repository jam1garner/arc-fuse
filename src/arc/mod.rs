#![allow(dead_code)]
use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io;
use std::mem;

mod util;
mod structs;
use crc::crc32;
use structs::*;
use memmap::Mmap;
use packed_struct::prelude::*;

static HASH_STRINGS: &'static str = include_str!("hashes.txt");

#[derive(Debug, Clone)]
pub enum ArcFileInfo {
    Uncompressed {
        offset: u64,
        size: u64,
        flags: u32,
    },
    Directory,
    None
}

pub struct Arc {
    pub file: File,
    pub map: Mmap,
    pub decomp_table: Vec<u8>,
    pub stream_entries: Vec<StreamEntry>,
    pub stream_file_indices: Vec<u32>,
    pub stream_offset_entries: Vec<StreamOffsetEntry>,
    pub names: HashMap<u64, &'static str>,
    pub stream_paths: HashMap<u64, &'static str>,
    pub dir_children: HashMap<u64, Vec<u64>>,
    pub files: HashMap<u64, ArcFileInfo>,
    pub stems: HashMap<u64, &'static str>,
    pub file_infos: Vec<FileInformationPath>,
    pub file_info_indices: Vec<FileInformationIndex>,
    pub directories: Vec<DirectoryInfo>,
    pub offsets1: Vec<DirectoryOffsets>,
    pub offsets2: Vec<DirectoryOffsets>,
    pub hash_folder_counts: Vec<FolderHashIndex>,
    pub file_infos_v2: Vec<FileInfo2>,
    pub file_info_sub_index: Vec<FileInfoSubIndex>,
    pub sub_file_info_start: usize,
    pub sub_file_infos1: Vec<SubFileInfo>,
    pub sub_file_info_start2: usize,
    pub sub_file_infos2: Vec<SubFileInfo>,
}

impl<'a> Arc {
    #[allow(unused_variables)]
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
            file_infos: vec![],
            stream_paths: HashMap::new(),
            names: HashMap::new(),
            dir_children: HashMap::new(),
            files: HashMap::new(),
            stems: HashMap::new(),
            file_info_indices: vec![],
            directories: vec![],
            offsets1: vec![],
            offsets2: vec![],
            hash_folder_counts: vec![],
            file_infos_v2: vec![],
            file_info_sub_index: vec![],
            sub_file_info_start: 0,
            sub_file_infos1: vec![],
            sub_file_info_start2: 0,
            sub_file_infos2: vec![],
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
            );
            ($n:expr) => (
                pos += $n;
            );
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
        let unk_counts = read!(u32, 2);
        skip!(8, (unk_counts[0] as usize + unk_counts[1] as usize));

        arc.file_infos = Vec::from(read!(FileInformationPath, node_header.file_info_count as usize));
        arc.file_info_indices = Vec::from(read!(FileInformationIndex, node_header.unk_offset_size_count as usize));
        let some_folder_thing = Vec::from(read!(SomeFolderThing, node_header.folder_count as usize));
        arc.directories = Vec::from(read!(DirectoryInfo, node_header.folder_count as usize));
        arc.offsets1 = Vec::from(read!(DirectoryOffsets, node_header.file_count1 as usize));
        arc.offsets2 = Vec::from(read!(DirectoryOffsets, node_header.file_count2 as usize));
        arc.hash_folder_counts = Vec::from(read!(FolderHashIndex, node_header.hash_folder_count as usize));
        arc.file_infos_v2 = Vec::from(read!(FileInfo2, 
                                node_header.file_information_count as usize +
                                node_header.sub_file_count2 as usize
                            ));
        arc.file_info_sub_index = Vec::from(read!(FileInfoSubIndex,
                                node_header.last_table_count as usize +
                                node_header.sub_file_count2 as usize
                            ));
        arc.sub_file_info_start = pos;
        arc.sub_file_infos1 = Vec::from(read!(SubFileInfo, node_header.sub_file_count as usize));
        arc.sub_file_info_start2 = pos;
        arc.sub_file_infos2 = Vec::from(read!(SubFileInfo, node_header.sub_file_count2 as usize));

        arc.load_hashes();
        arc.load_stream_files();
        arc.load_compressed_files();
        // Arc tree
        // println!("Tree\n----");
        // arc.print_tree(0, 0);

        Ok(arc)
    }

    pub fn get_name(&self, hash40: u64) -> Option<&&str> {
        self.names.get(&hash40)
            .or(self.stream_paths.get(&hash40))
    }

    pub fn get_file_data(&self, hash40: u64) -> Option<&'a [u8]> {
        match self.files.get(&hash40) {
            Some(ArcFileInfo::Uncompressed {
                offset, size, flags: _
            }) => {
                unsafe {
                    Some(mem::transmute(
                        &self.map[*offset as usize..(offset+size) as usize]
                    ))
                }
            }
            _ => {
                eprintln!("File not found");
                None
            }
        }
    }

    fn print_tree(&self, node: u64, depth: u64) {
        for _ in 0..depth {
            print!("    ");
        }
        println!("{}", self.stems.get(&node).unwrap_or(&"error"));
        if let Some(ArcFileInfo::Directory) = self.files.get(&node) {
            for child in self.dir_children.get(&node).unwrap() {
                self.print_tree(*child, depth + 1);
            }
        }
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
                Arc::hash40(line),
                line
            );
        }
    }

    pub fn hash40<S: AsRef<str>>(string: S) -> u64 {
        crc32::checksum_ieee(string.as_ref().as_bytes()) as u64 +
            ((string.as_ref().len() as u64) << 32)
    }

    fn add_dir(dirs: &mut HashMap<u64, Vec<u64>>, files: &mut HashMap<u64, ArcFileInfo>,
               stems: &mut HashMap<u64, &'static str>, names: &mut HashMap<u64, &'static str>,
               parent: &'static str, dir: &'static str) {
        let parent_hash40 = Arc::hash40(parent);
        let dir_hash40 = Arc::hash40(dir);
        if !dirs.contains_key(&parent_hash40) {
            dirs.insert(parent_hash40, vec![]);
        }
        let parent_children = dirs.get_mut(&parent_hash40).unwrap();
        if dir_hash40 != 0 && !parent_children.contains(&dir_hash40) {
            parent_children.push(dir_hash40);
        }

        if !dirs.contains_key(&dir_hash40) {
            dirs.insert(dir_hash40, vec![]);
        }
        files.insert(dir_hash40, ArcFileInfo::Directory);
        stems.insert(dir_hash40, dir.split("/").last().unwrap());
        names.insert(dir_hash40, dir);
    }

    fn load_stream_files(&mut self) {
        self.dir_children.insert(0, vec![]);
        self.names.insert(0, "");
        self.stems.insert(0, "");
        self.files.insert(0, ArcFileInfo::Directory);
        for stream_file in &self.stream_entries {
            if let Some(path) = self.names.get(&(
                stream_file.hash as u64 + ((stream_file.name_length as u64) << 32)
            )) {
                let path_components: Vec<&'static str> = path.split("/").collect();
                let mut pos = 0;
                let mut last: &'static str = "";
                for dir in &path_components[..path_components.len()-1] {
                    let dir_len = dir.len();
                    let current = &path[0..pos + dir_len];
                    
                    // Add directory
                    Arc::add_dir(&mut self.dir_children, &mut self.files, &mut self.stems,
                                 &mut self.stream_paths, last, current);
                    
                    last = current;
                    pos += 1 + dir_len;
                }
                let stream_offset_entry = self.stream_offset_entries[
                    self.stream_file_indices[stream_file.index as usize] as usize
                ];
                let file_hash40 = Arc::hash40(path);
                self.files.insert(
                    file_hash40,
                    ArcFileInfo::Uncompressed {
                        offset: stream_offset_entry.offset,
                        size: stream_offset_entry.size,
                        flags: stream_file.flags,
                    }
                );
                self.dir_children
                    .get_mut(&Arc::hash40(last))
                    .unwrap()
                    .push(file_hash40);
                self.stems.insert(
                    file_hash40,
                    path_components[path_components.len() - 1]
                );
            }
        }
    }

    fn load_compressed_files(&mut self) {

    }
}

