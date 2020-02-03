#![allow(dead_code)]
use std::collections::BTreeMap;
use std::path::Path;
use std::fs::File;
use std::io;

mod util;
mod structs;
mod mem_file;
use mem_file::{set_file, get_header, FilePtr64, FileSlice};
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

pub struct ArcInternal<'a> {
    pub stream_entries: Vec<StreamEntry>,
    pub stream_file_indices: &'a [u32],
    pub stream_offset_entries: &'a [StreamOffsetEntry],
    pub file_infos: &'a [FileInformationPath],
    pub file_info_indices: &'a [FileInformationIndex],
    pub dir_hash_to_index: &'a [SomeFolderThing],
    pub directories: &'a [DirectoryInfo],
    pub offsets1: &'a [DirectoryOffsets],
    pub offsets2: &'a [DirectoryOffsets],
    pub hash_folder_counts: &'a [FolderHashIndex],
    pub file_infos_v2: &'a [FileInfo2],
    pub file_info_sub_index: &'a [FileInfoSubIndex],
    pub sub_file_info_start: usize,
    pub sub_file_infos1: &'a [SubFileInfo],
    pub sub_file_info_start2: usize,
    pub sub_file_infos2: &'a [SubFileInfo],
}

pub struct Arc {
    pub file: File,
    pub map: Mmap,
    pub names: BTreeMap<u64, &'static str>,
    pub stream_paths: BTreeMap<u64, &'static str>,
    pub dir_children: BTreeMap<u64, Vec<u64>>,
    pub files: BTreeMap<u64, ArcFileInfo>,
    pub stems: BTreeMap<u64, &'static str>,

    //pub decomp_table: Vec<u8>,
}

impl<'a> Arc {
    #[allow(unused_variables)]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let file = File::open(path.as_ref())?;
        let map = unsafe { Mmap::map(&file) }?;

        let mut arc = Arc {
            file,
            map,
            stream_paths: BTreeMap::new(),
            names: BTreeMap::new(),
            dir_children: BTreeMap::new(),
            files: BTreeMap::new(),
            stems: BTreeMap::new(),
        };

        set_file(&*arc.map);

        let decomp_table = arc.decompress_table().unwrap();

        set_file(&decomp_table);

        let node_header = get_header::<NodeHeader>();
        let node_header_2 = FilePtr64::<NodeHeader2>::new(0x100);
        
        let count = node_header_2.part1_count as usize;
        let something = node_header_2.next_slice::<u64>(count);
        let stream_entries_ptr = something.next_slice::<[u8; 0xC]>(count);
        let stream_entries = stream_entries_ptr.iter()
                                .map(|a| StreamEntry::unpack(a))
                                .collect::<Result<Vec<_>, _>>().unwrap();

        let count = node_header_2.stream_file_index_count as usize;
        let stream_file_indices = stream_entries_ptr.next_slice::<u32>(count);

        let count = node_header_2.stream_offset_entry_count as usize;
        let stream_offset_entries = stream_file_indices.next_slice::<StreamOffsetEntry>(count);

        let unk_counts = stream_offset_entries.next::<[u32; 2]>();

        let unk1 = unk_counts.next_slice::<u64>(unk_counts[0] as usize);
        let unk2 = unk1.next_slice::<u64>(unk_counts[1] as usize);

        let file_infos = unk2.next_slice::<FileInformationPath>(node_header.file_info_count as _);

        let count = node_header.unk_offset_size_count as usize;
        let file_info_indices = file_infos.next_slice::<FileInformationIndex>(count);

        let folder_count = node_header.folder_count as usize;
        let dir_hash_to_index = file_info_indices.next_slice::<SomeFolderThing>(folder_count);

        let dirs = dir_hash_to_index.next_slice::<DirectoryInfo>(folder_count);

        let offsets1 = dirs.next_slice::<DirectoryOffsets>(node_header.file_count1 as _);
        let offsets2 = offsets1.next_slice::<DirectoryOffsets>(node_header.file_count2 as _);

        let count = node_header.hash_folder_count as usize;
        let hash_folder_counts = offsets2.next_slice::<FolderHashIndex>(count);

        let count = node_header.file_information_count as usize + node_header.sub_file_count2 as usize;
        let file_infos_v2 = hash_folder_counts.next_slice::<FileInfo2>(count);

        let count = node_header.last_table_count as usize + node_header.sub_file_count2 as usize;
        let file_info_sub_index = file_infos_v2.next_slice::<FileInfoSubIndex>(count);

        let count = node_header.sub_file_count as usize;
        let sub_file_infos1 = file_info_sub_index.next_slice::<SubFileInfo>(count);
        let sub_file_info_start = sub_file_infos1.inner_ptr();

        let count = node_header.sub_file_count2 as usize;
        let sub_file_infos2 = sub_file_infos1.next_slice::<SubFileInfo>(count);
        let sub_file_info_start2 = sub_file_infos2.inner_ptr();

        let arc_internal = ArcInternal {
            dir_hash_to_index: &dir_hash_to_index,
            directories: &dirs,
            file_info_indices: &file_info_indices,
            file_info_sub_index: &file_info_sub_index,
            file_infos: &file_infos,
            file_infos_v2: &file_infos_v2,
            hash_folder_counts: &hash_folder_counts,
            offsets1: &offsets1,
            offsets2: &offsets2,
            sub_file_info_start: sub_file_info_start,
            sub_file_infos1: &sub_file_infos1,
            sub_file_info_start2: sub_file_info_start2,
            sub_file_infos2: &sub_file_infos2,
            stream_entries: stream_entries,
            stream_file_indices: &stream_file_indices,
            stream_offset_entries: &stream_offset_entries,
        };

        set_file(&arc.map);

        arc.load_hashes();
        arc.load_stream_files(&arc_internal);
        arc.load_compressed_files();
        // Arc tree
        // println!("Tree\n----");
        // arc.print_tree(0, 0);

        Ok(arc)
    }

    pub fn get_name(&self, hash40: u64) -> Option<&&str> {
        self.names.get(&hash40)
            .or_else(||self.stream_paths.get(&hash40))
    }

    pub fn get_file_data(&self, hash40: u64) -> Option<FileSlice<u8>> {
        match self.files.get(&hash40) {
            Some(&ArcFileInfo::Uncompressed {
                offset, size, ..
            }) => {
                Some(FileSlice::<u8>::new(offset as _, size as _))
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

    fn compressed_table(&self) -> FileSlice<u8> {
        let header = get_header::<ArcHeader>();
        let comp_table_hdr = &header.comp_table_header;
        comp_table_hdr.next_slice(comp_table_hdr.comp_size as _)
    }

    fn decompress_table(&mut self) -> io::Result<Vec<u8>> {
        let compressed_table = self.compressed_table();
        let compressed_table = io::Cursor::new(&*compressed_table);
        zstd::stream::decode_all(compressed_table)
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

    fn add_dir(dirs: &mut BTreeMap<u64, Vec<u64>>, files: &mut BTreeMap<u64, ArcFileInfo>,
               stems: &mut BTreeMap<u64, &'static str>, names: &mut BTreeMap<u64, &'static str>,
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

    fn load_stream_files(&mut self, arc: &ArcInternal) {
        self.dir_children.insert(0, vec![]);
        self.names.insert(0, "");
        self.stems.insert(0, "");
        self.files.insert(0, ArcFileInfo::Directory);
        for stream_file in &arc.stream_entries {
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
                let stream_offset_entry = arc.stream_offset_entries[
                    arc.stream_file_indices[stream_file.index as usize] as usize
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

