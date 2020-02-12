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

static HASH_STRINGS: &'static str = include_str!("hash40s.tsv");

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
    pub dir_hash_to_index: &'a [HashIndexGroup],
    pub directories: &'a [DirectoryInfo],
    pub file_infos_v2: &'a [FileInfo2],
    pub file_info_sub_index: &'a [FileInfoSubIndex],
    pub sub_files: &'a [SubFileInfo],
    pub quick_dirs: &'a [QuickDir],
    /*pub offsets1: &'a [DirectoryOffsets],
    pub offsets2: &'a [DirectoryOffsets],
    pub hash_folder_counts: &'a [FolderHashIndex],
    pub sub_file_info_start: usize,
    pub sub_file_infos1: &'a [SubFileInfo],
    pub sub_file_info_start2: usize,
    pub sub_file_infos2: &'a [SubFileInfo],*/
}

pub struct Arc {
    pub file: File,
    pub map: Mmap,
    pub names: BTreeMap<u64, &'static str>,
    pub stream_paths: BTreeMap<u64, &'static str>,
    pub dir_children: BTreeMap<u64, Vec<u64>>,
    pub files: BTreeMap<u64, ArcFileInfo>,
    pub stems: BTreeMap<u64, &'static str>,
}

impl<'a> Arc {
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

        let fs_header = get_header::<FileSystemHeader>();
        let stream_header = FilePtr64::<StreamHeader>::new(0x100);

        dbg!(stream_header.quick_dir_count);

        // ArcCross names:
        // streamUnk
        let quick_dirs_ptr = stream_header.next_slice::<[u8; 0xC]>(stream_header.quick_dir_count as _);
        let quick_dirs = quick_dirs_ptr.iter()
                            .map(|a| QuickDir::unpack(a))
                            .collect::<Result<Vec<_>, _>>().unwrap();
        //println!("streamUnk: {:X}", quick_dirs_ptr.inner_ptr());
        // streamHashToName
        let count = stream_header.stream_hash_count as usize;
        let stream_hashes = quick_dirs_ptr.next_slice::<u64>(count);
        //println!("streamHashToName: {:X}", stream_hashes.inner_ptr());

        // streamNameToHash
        let stream_entries_ptr = stream_hashes.next_slice::<[u8; 0xC]>(count);
        let stream_entries = stream_entries_ptr.iter()
                                .map(|a| StreamEntry::unpack(a))
                                .collect::<Result<Vec<_>, _>>().unwrap();
        //println!("streamNameToHash: {:X}", stream_entries_ptr.inner_ptr());

        // streamIndexToFile
        let count = stream_header.stream_file_index_count as usize;
        let stream_file_indices = stream_entries_ptr.next_slice::<u32>(count);
        //println!("streamIndexToFile: {:X}", stream_file_indices.inner_ptr());

        // streamOffsets
        let count = stream_header.stream_offset_entry_count as usize;
        let stream_offset_entries = stream_file_indices.next_slice::<StreamOffsetEntry>(count);
        //println!("streamOffsets: {:X}", stream_offset_entries.inner_ptr());

        // ----- Compressed stuff ------
        // unkCount1, unkCount2
        let unk_counts = stream_offset_entries.next::<[u32; 2]>();
        dbg!(&*unk_counts);
        //println!("unkCount1: {:X}", unk_counts.inner());

        // fileInfoUnknownTable
        let file_info_unks = unk_counts.next_slice::<FileInformationUnknownTable>(unk_counts[1] as usize);
        //println!("fileInfoUnknownTable: {:X}", file_info_unks.inner_ptr());

        // filePathToIndexHashGroup
        let hash_index_groups = file_info_unks.next_slice::<HashIndexGroup>(unk_counts[0] as usize);
        //println!("filePathToIndexHashGroup: {:X}", hash_index_groups.inner_ptr());

        // fileInfoPath
        let file_infos = hash_index_groups
                            .next_slice::<FileInformationPath>(fs_header.file_info_path_count as _);
        //println!("fileInfoPath: {:X}", file_infos.inner_ptr());

        // fileInfoIndex
        let count = fs_header.file_info_index_count as usize;
        let file_info_indices = file_infos.next_slice::<FileInformationIndex>(count);
        //println!("fileInfoIndex: {:X}", file_info_indices.inner_ptr());

        // directoryHashGroup
        let folder_count = fs_header.folder_count as usize;
        let dir_hash_to_index = file_info_indices.next_slice::<HashIndexGroup>(folder_count);
        //println!("directoryHashGroup: {:X}", dir_hash_to_index.inner_ptr());

        // directoryList
        let dirs = dir_hash_to_index.next_slice::<DirectoryInfo>(folder_count);
        //println!("directoryList: {:X}", dirs.inner_ptr());

        use std::mem::size_of;
        dbg!(size_of::<DirectoryOffsets>());

        // directoryOffsets
        let folder_offsets = dirs.next_slice::<DirectoryOffsets>(
            dbg!(fs_header.folder_offset_count_1) as usize +
            dbg!(fs_header.folder_offset_count_2) as usize +
            dbg!(fs_header.extra_folder) as usize
        );
        //println!("directoryOffsets: {:X}", folder_offsets.inner_ptr());

        // directoryChildHashGroup
        let count = fs_header.hash_folder_count as usize;
        let folder_child_hashes = folder_offsets.next_slice::<HashIndexGroup>(count);
        //println!("directoryChildHashGroup: {:X}", folder_child_hashes.inner_ptr());

        // fileInfoV2
        let count = fs_header.file_info_count as usize +
                    fs_header.sub_file_count_2 as usize +
                    fs_header.extra_count as usize;
        let file_infos_v2 = folder_child_hashes.next_slice::<FileInfo2>(count);
        //println!("fileInfoV2: {:X}", file_infos_v2.inner_ptr());

        // fileInfoSubIndex
        let count = fs_header.file_info_sub_index_count as usize +
                    fs_header.sub_file_count_2 as usize +
                    fs_header.extra_count_2 as usize;
        let file_info_sub_index = file_infos_v2.next_slice::<FileInfoSubIndex>(count);
        //println!("fileInfoSubIndex: {:X}", file_info_sub_index.inner_ptr());

        // subFiles
        let count = fs_header.file_info_sub_index_count as usize +
                    fs_header.sub_file_count_2 as usize +
                    fs_header.extra_count_2 as usize;
        let sub_files = file_info_sub_index.next_slice::<SubFileInfo>(count);
        //println!("subFiles: {:X}", sub_files.inner_ptr());

        // stream_entries, stream_offest_entries, stream_file_indices
        let arc_internal = ArcInternal {
            dir_hash_to_index: &dir_hash_to_index,
            directories: &dirs,
            file_info_indices: &file_info_indices,
            file_info_sub_index: &file_info_sub_index,
            file_infos: &file_infos,
            file_infos_v2: &file_infos_v2,
            sub_files: &sub_files,
            stream_entries: stream_entries,
            stream_file_indices: &stream_file_indices,
            stream_offset_entries: &stream_offset_entries,
            quick_dirs: &quick_dirs,
        };

        set_file(&arc.map);

        arc.load_hashes();
        arc.load_stream_files(&arc_internal);
        arc.load_compressed_files(&arc_internal);
        // Arc tree
        // println!("Tree\n----");
        // arc.print_tree(0, 0);

        Ok(arc)
    }

    pub fn get_name(&self, hash40: u64) -> Option<&str> {
        self.names.get(&hash40)
            .or_else(||self.stream_paths.get(&hash40))
            .map(std::ops::Deref::deref)
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
        let arc = get_header::<ArcHeader>();
        let comp_table_hdr = &arc.file_system;
        comp_table_hdr.next_slice(comp_table_hdr.comp_size as _)
    }

    fn decompress_table(&mut self) -> io::Result<Vec<u8>> {
        let compressed_table = self.compressed_table();
        let compressed_table = io::Cursor::new(&*compressed_table);
        zstd::stream::decode_all(compressed_table)
    }
    
    fn load_hashes(&mut self) {
        self.names = HASH_STRINGS.split('\n')
            .filter_map(|line|{
                let split: Vec<&'static str> = line.split('\t').collect();
                if let &[hash, string] = &split[..] {
                    Some((u64::from_str_radix(hash, 16).ok()?, string))
                } else {
                    None
                }
            })
            .collect();
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
            let hash40 =stream_file.hash as u64 + ((stream_file.name_length as u64) << 32);
            if let Some(path) = self.names.get(&(
                hash40
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
            } else {
                println!("Warning: hash 0x{:X} not found", hash40);
            }
        }
    }

    fn load_compressed_files(&mut self, arc: &ArcInternal) {
    }
}

