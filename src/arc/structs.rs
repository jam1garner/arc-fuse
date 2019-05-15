#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArcHeader {
    pub magic: u64, // 0xABCDEF9876543210
    pub music_section_offset: u64,
    pub file_section_offset: u64,
    pub file2_section_offset: u64,
    pub node_section_offset: u64,
    pub unk_section_offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CompTableHeader {
    pub header_size: u32, // 0x10
    pub unk: u32,
    pub comp_size: u32,
    pub next_table: u32,
}


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NodeHeader {
    pub filesize: u32,
    pub file_info_count: u32,
    pub unk_offset_size_count: u32,
    pub folder_count: u32,

    pub file_count1: u32,

    pub hash_folder_count: u32,
    pub file_information_count: u32,
    pub last_table_count: u32,
    pub sub_file_count: u32,

    pub file_count2: u32,
    pub sub_file_count2: u32,
    pub unk11: u32,
    pub unk1_10: u32,
    pub unk2_10: u32,
    pub unk13: u32,
    pub unk14: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NodeHeader2 {
    pub unk3: u32,
    pub part1_count: u32,
    pub part2_count: u32,
    pub part3_count: u32,

    pub unk5: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,

    pub unk9: u32,
    pub unk10: u32,
    pub unk11: u32,

    pub unk12: u32,
    pub unk13: u32,
}

#[derive(Debug, Clone, Copy, PackedStruct)]
#[packed_struct(endian="lsb", bit_numbering="msb0")]
pub struct StreamEntry {
    #[packed_field(bits="0..32")]
    pub hash: u32,
    #[packed_field(bits="32..40")]
    pub name_length: u8,
    #[packed_field(bits="40..64")]
    pub index: u32,
    #[packed_field(bits="64..96")]
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamOffsetEntry {
    pub size: u64,
    pub offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInformationPath {
    pub path: u32,
    pub directory_index: u32,
    pub extension: u32,
    pub file_table_path: u32,
    pub parent: u32,
    pub unk5: u32,
    pub hash2: u32,
    pub unk6: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInformationIndex {
    pub some_indices: [u32; 2]
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SomeFolderThing {
    pub hash: u32,
    pub unk: u8,
    pub index: u16,
    pub padding: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Hash40 {
    pub hash: u32,
    pub length: u8,
    pub padding: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DirectoryInfo {
    pub path_hash: u32,
    pub dir_offset_index: u32,
    pub name: Hash40,
    pub parent: Hash40,
    pub extra_dis_re: u32,
    pub extra_dis_re_length: u32,
    pub file_name_start_index: u32,
    pub file_info_count: u32,
    pub child_dir_start_index: u32,
    pub child_dir_count: u32,
    pub flags: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DirectoryOffsets {
    pub offset: u64,
    pub decomp_size: u32,
    pub size: u32,
    pub sub_data_start_index: u32,
    pub sub_data_count: u32,
    pub resource_index: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FolderHashIndex {
    pub hash: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInfo2 {
    pub hash_index: u32,
    pub hash_index_2: u32,
    pub sub_file_index: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInfoSubIndex {
    pub some_indices:[u32; 3]
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubFileInfo {
    pub offset: u32,
    pub comp_size: u32,
    pub decomp_size: u32,
    pub flags: u32,
}
