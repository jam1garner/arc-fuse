#![allow(dead_code)]
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::path::Path;
use std::fs::File;
use std::io;

use rayon::prelude::*;

mod util;
mod structs;
mod mem_file;
use mem_file::{set_file, get_header, FilePtr64, FileSlice};
use crc::crc32::checksum_ieee as crc32;
use structs::*;
use memmap::Mmap;
use packed_struct::prelude::*;
use cached::{cached_key, SizedCache};

static HASH_STRINGS: ArcStr = include_str!("hash40s.tsv");

//include!(concat!(env!("OUT_DIR"), "/hash40s.rs"));

cached_key!{
    FILE_CACHE: SizedCache<u64, Option<Vec<u8>>> = SizedCache::with_size(50);
    Key = { hash40 };
    fn decompress_file(hash40: u64, file: FileSlice<u8>) -> Option<Vec<u8>> = {
        let reader = io::Cursor::new(&*file);
        zstd::decode_all(reader).ok()
    }
}

pub fn hash40(string: &str) -> u64 {
    crc32(string.as_bytes()) as u64 +
        ((string.len() as u64) << 32)
}

#[derive(Debug, Clone)]
pub enum ArcFileInfo {
    Uncompressed {
        data: FileSlice<u8>,
        flags: u32,
    },
    Compressed {
        data: FileSlice<u8>,
        decomp_size: u64,
    },
    Directory,
    None
}

type ArcStr = &'static str;

pub struct ArcInternal<'a> {
    pub arc_header: &'a ArcHeader,
    pub stream_entries: Vec<StreamEntry>,
    pub stream_file_indices: &'a [u32],
    pub stream_offset_entries: &'a [StreamOffsetEntry],
    pub file_info_paths: &'a [FileInformationPath],
    pub file_info_indices: &'a [FileInformationIndex],
    pub dir_hash_to_index: &'a [HashIndexGroup],
    pub directories: &'a [DirectoryInfo],
    pub file_infos_v2: &'a [FileInfo2],
    pub file_info_sub_index: &'a [FileInfoSubIndex],
    pub sub_files: &'a [SubFileInfo],
    pub quick_dirs: &'a [QuickDir],
    pub folder_offsets: &'a [DirectoryOffsets],
}

pub struct Arc {
    pub file: File,
    pub map: Mmap,
    pub names: HashMap<u64, ArcStr>,
    pub stream_paths: HashMap<u64, ArcStr>,
    pub dir_children: HashMap<u64, HashSet<u64>>,
    pub files: HashMap<u64, ArcFileInfo>,
    pub stems: HashMap<u64, ArcStr>,
}

impl<'a> Arc {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let file = File::open(path.as_ref())?;
        let map = unsafe { Mmap::map(&file) }?;

        let mut arc = Arc {
            file,
            map,
            stream_paths: HashMap::new(),
            names: HashMap::new(),
            dir_children: HashMap::new(),
            files: HashMap::new(),
            stems: HashMap::new(),
        };

        set_file(&*arc.map);

        let arc_header = &*get_header::<ArcHeader>();
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
        let stream_entries = stream_entries_ptr
                                .iter()
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
        std::fs::write("fileInfoUnknownTable.bin", &*file_info_unks.as_byte_file_slice()).unwrap();
        //println!("fileInfoUnknownTable: {:X}", file_info_unks.inner_ptr());

        // filePathToIndexHashGroup
        let hash_index_groups = file_info_unks.next_slice::<HashIndexGroup>(unk_counts[0] as usize);
        std::fs::write("filePathToIndexHashGroup.bin", &*hash_index_groups.as_byte_file_slice()).unwrap();
        //println!("filePathToIndexHashGroup: {:X}", hash_index_groups.inner_ptr());

        // fileInfoPath
        let file_info_paths = hash_index_groups
                            .next_slice::<FileInformationPath>(fs_header.file_info_path_count as _);
        //println!("fileInfoPath: {:X}", file_infos.inner_ptr());

        // fileInfoIndex
        let count = fs_header.file_info_index_count as usize;
        let file_info_indices = file_info_paths.next_slice::<FileInformationIndex>(count);
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
        let count = fs_header.sub_file_count as usize +
                    fs_header.sub_file_count_2 as usize +
                    fs_header.extra_count as usize;
        let sub_files = file_info_sub_index.next_slice::<SubFileInfo>(count);
        //println!("subFiles: {:X}", sub_files.inner_ptr());
        let end = sub_files.next::<()>().inner();
        dbg!(end);
        dbg!(*fs_header);

        // stream_entries, stream_offest_entries, stream_file_indices
        let arc_internal = ArcInternal {
            arc_header,
            dir_hash_to_index: &dir_hash_to_index,
            directories: &dirs,
            file_info_indices: &file_info_indices,
            file_info_sub_index: &file_info_sub_index,
            file_info_paths: &file_info_paths,
            file_infos_v2: &file_infos_v2,
            sub_files: &sub_files,
            stream_entries: stream_entries,
            stream_file_indices: &stream_file_indices,
            stream_offset_entries: &stream_offset_entries,
            quick_dirs: &quick_dirs,
            folder_offsets: &folder_offsets,
        };
        

        arc.load_hashes();
        arc.load_stream_files(&arc_internal);
        use timeit::*;
        let load_time = timeit_loops!(1, {
            arc.load_compressed_files(&arc_internal);
        });
        dbg!(load_time);
        set_file(&*arc.map);

        // Arc tree
        // println!("Tree\n----");
        // arc.print_tree(0, 0);

        Ok(arc)
    }

    pub fn get_name(&self, hash40: u64) -> Option<&'static str> {
        let x = self.names.get(&hash40);
        let x = if let Some(x) = x {
            Some(x)
        } else {
            self.stream_paths.get(&hash40)
        };

        x.map(|a| *a)
    }

    pub fn get_file_data(&self, hash40: u64) -> Option<FileSliceOrVec> {
        match self.files.get(&hash40) {
            Some(&ArcFileInfo::Uncompressed {
                data, ..
            }) => {
                Some(FileSliceOrVec::FileSlice(data))
            }
            Some(&ArcFileInfo::Compressed {
                data, decomp_size
            }) => {
                if data.len() == decomp_size as usize {
                    Some(FileSliceOrVec::FileSlice(data))
                } else {
                    let f = decompress_file(hash40, data)?;
                    Some(FileSliceOrVec::Vec(f))
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
        self.names =
            HASH_STRINGS
                .par_split('\n')
                .filter_map(|line|{
                    let split: Vec<ArcStr> = line.split('\t').collect();
                    if let &[hash, string] = &split[..] {
                        Some((u64::from_str_radix(hash, 16).ok()?, string))
                    } else {
                        None
                    }
                })
                .collect();
    }

    fn add_dir(&mut self, parent: ArcStr, dir: ArcStr) -> u64 {
        let dirs = &mut self.dir_children;
        let (parent_hash40, dir_hash40) = (hash40(parent), hash40(dir));
        if !dirs.contains_key(&parent_hash40) {
            dirs.insert(parent_hash40, HashSet::new());
        }
        let parent_children = dirs.get_mut(&parent_hash40).unwrap();
        if dir_hash40 != 0 {
            parent_children.insert(dir_hash40);
        }

        if !dirs.contains_key(&dir_hash40) {
            dirs.insert(dir_hash40, HashSet::new());
        }

        self.stems.insert(dir_hash40, dir.rsplit("/").nth(0).unwrap());
        self.stream_paths.insert(dir_hash40, dir);
        self.files.insert(dir_hash40, ArcFileInfo::Directory);

        dir_hash40
    }

    fn add_dirs(&mut self, path: ArcStr, path_components: &Vec<ArcStr>) -> u64 {
        let mut pos = 0;
        let mut last: ArcStr = "";
        let mut last_hash = std::u64::MAX;
        for dir in path_components.split_last().unwrap().1 {
            let dir_len = dir.len();
            let current = &path[0..pos + dir_len];
            
            last_hash = self.add_dir(last, current);
            
            last = current;
            pos += 1 + dir_len;
        }
        last_hash
    }

    fn load_stream_files(&mut self, arc: &ArcInternal) {
        self.dir_children.insert(0, HashSet::new());
        self.names.insert(0, "");
        self.stems.insert(0, "");
        self.files.insert(0, ArcFileInfo::Directory);
        for stream_file in &arc.stream_entries {
            let hash40 = stream_file.hash as u64 + ((stream_file.name_length as u64) << 32);
            if let Some(path) = self.get_name(hash40) {
                let path_components = path.split('/').collect();
                let last = self.add_dirs(path, &path_components);
                let stream_offset_entry = arc.stream_offset_entries[
                    arc.stream_file_indices[stream_file.index as usize] as usize
                ];
                let (offset, size) = (stream_offset_entry.offset as usize, stream_offset_entry.size as usize);
                self.files.insert(
                    hash40,
                    ArcFileInfo::Uncompressed {
                        data: FileSlice::new(offset, size),
                        flags: stream_file.flags,
                    }
                );
                self.dir_children
                    .get_mut(&last)
                    .unwrap()
                    .insert(hash40);
                self.stems.insert(
                    hash40,
                    path_components.last().unwrap()
                );
            } else {
                println!("Warning: hash 0x{:X} not found", hash40);
            }
        }
    }

    fn get_file_compressed(arc: &ArcInternal, file_info: &FileInfo2) -> (FileSlice<u8>, u64) {
        let file_index = arc.file_info_indices[file_info.hash_index_2 as usize];
        let file_info = if file_info.flags & REDIRECT != 0 {
            &arc.file_infos_v2[file_index.file_info_index as usize]
        } else {
            file_info
        };

        let sub_index = arc.file_info_sub_index[file_info.sub_file_index as usize];
        
        let sub_file = arc.sub_files[sub_index.sub_file_index as usize];
        let dir_offset = arc.folder_offsets[sub_index.folder_offset_index as usize];

        let offset = arc.arc_header.file_section_offset as usize +
                        dir_offset.offset as usize +
                        ((sub_file.offset as usize) << 2);

        (FileSlice::new(offset, sub_file.comp_size as usize), sub_file.decomp_size as u64)
    }

    fn load_compressed_files(&mut self, arc: &ArcInternal) {
        let file_infos: Vec<_> =
            arc.file_infos_v2
                .par_iter()
                .filter_map(|file_info|{
                    let path = arc.file_info_paths[file_info.hash_index as usize];
                    let file_hash40 = path.path.hash40();
                    let path_string = self.get_name(file_hash40)?;
                    let path_components: Vec<_> = path_string.split('/').collect();
                    let (data, decomp_size) = Arc::get_file_compressed(arc, file_info);

                    Some((file_hash40, path_string, path_components, data, decomp_size))
                })
                .collect();

        let file_infos: Vec<_> =
            file_infos
                .into_iter()
                .map(|(file_hash40, path_string, path_components, data, decomp_size)|{
                    let last = self.add_dirs(path_string, &path_components);
                    (file_hash40, path_string, path_components, data, decomp_size, last)
                })
                .collect();

        let dir_children = Mutex::new(&mut self.dir_children);
        let stems = Mutex::new(&mut self.stems);
        let files = Mutex::new(&mut self.files);
        rayon::join(
            || rayon::join(
                || {
                    files
                        .lock()
                        .unwrap()
                        .par_extend(
                            file_infos
                                .par_iter()
                                .map(|&(hash40, .., data, decomp_size, _)|(
                                    hash40,
                                    ArcFileInfo::Compressed {
                                        data, decomp_size
                                    }
                                ))
                        );
                },
                || {
                    stems
                        .lock()
                        .unwrap()
                        .par_extend(
                            file_infos
                                .par_iter()
                                .map(|&(hash40, _, ref path_components, ..)| {(
                                    hash40,
                                    *path_components.last().unwrap()
                                )})
                        );
                }
            ),
            || {
                file_infos
                    .par_iter()
                    .for_each(|&(hash40, .., last )|{
                        dir_children
                            .lock()
                            .unwrap()
                            .get_mut(&last)
                            .unwrap()
                            .insert(hash40);
                    });
            }
        );
    }
}

const REDIRECT: u32 = 0x00000010;

pub enum FileSliceOrVec {
    FileSlice(FileSlice<u8>),
    Vec(Vec<u8>)
}

impl FileSliceOrVec {
    pub fn get_slice(&self) -> &[u8] {
        match self {
            FileSliceOrVec::FileSlice(file_slice) => &file_slice,
            FileSliceOrVec::Vec(vec) => &vec
        }
    }
}
