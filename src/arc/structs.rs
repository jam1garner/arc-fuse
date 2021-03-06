use super::mem_file::FilePtr64;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArcHeader {
    pub magic: u64, // 0xABCDEF9876543210
    pub music_section_offset: u64, // offset1
    pub file_section_offset:  u64, // offset2
    pub shared_section_offset: u64, // offset3
    pub file_system: FilePtr64<CompTableHeader>, //offset4
    pub unk_section_offset: FilePtr64<CompTableHeader>, // offset5
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CompTableHeader {
    pub header_size: u32, // 0x10
    pub decomp_size: u32,
    pub comp_size: u32,
    pub section_size: u32,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileSystemHeader {
    pub table_filesize: u32,
    pub file_info_path_count: u32,
    pub file_info_index_count: u32,
    pub folder_count: u32,

    pub folder_offset_count_1: u32,

    pub hash_folder_count: u32,
    pub file_info_count: u32,
    pub file_info_sub_index_count: u32,
    pub sub_file_count: u32,

    pub folder_offset_count_2: u32,
    pub sub_file_count_2: u32,
    pub padding: u32,

    pub unk1_10: u32, // always 0x10
    pub unk2_10: u32, // always 0x10

    pub regional_count_1: u8,
    pub regional_count_2: u8,
    pub padding2: u16,
    
    pub version: u32,
    pub extra_folder: u32,
    pub extra_count: u32,

    pub unk: [u32; 2],

    pub extra_count_2: u32,
    pub extra_sub_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamHeader {
    pub quick_dir_count: u32,
    pub stream_hash_count: u32,
    pub stream_file_index_count: u32,
    pub stream_offset_entry_count: u32,
}

#[derive(Debug, Clone, Copy, PackedStruct)]
#[packed_struct(endian="lsb", bit_numbering="msb0")]
pub struct QuickDir {
    #[packed_field(bits="0..32")]
    pub hash: u32,
    #[packed_field(bits="32..40")]
    pub name_length: u8,
    #[packed_field(bits="40..64")]
    pub count: u32,
    #[packed_field(bits="64..96")]
    pub index: u32,
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

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileInformationPath {
    pub path: HashIndexGroup,
    pub ext: HashIndexGroup,
    pub parent: HashIndexGroup,
    pub file_name: HashIndexGroup,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInformationIndex {
    pub dir_offset_index: u32,
    pub file_info_index: u32,
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

#[repr(packed)]
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
    // PathIndex
    pub hash_index: u32,
    // IndexIndex
    pub hash_index_2: u32,
    // SubIndexIndex
    pub sub_file_index: u32,
    // Flags
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInfoSubIndex {
    pub folder_offset_index: u32,
    pub sub_file_index: u32,
    pub file_info_index_and_flag: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubFileInfo {
    pub offset: u32,
    pub comp_size: u32,
    pub decomp_size: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamHashToName {
    pub hash: u32,
    pub name_index: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileInformationUnknownTable {
    pub some_index: u32,
    pub some_index_2: u32,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct HashIndexGroup {
    pub hash: u32,
    pub index: u32,
}

impl HashIndexGroup {
    pub fn hash40(&self) -> u64 {
        self.hash as u64 + ((self.index as u64 & 0xFF) << 32)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_sizes() {
        use std::mem::size_of;
        let size_of_quick_dir = 12;
        dbg!(
            size_of::<ArcHeader>(),
            size_of::<CompTableHeader>(),
            size_of::<FileSystemHeader>(),
            size_of::<StreamHeader>(),
            size_of_quick_dir,
            size_of::<StreamEntry>(),
            size_of::<StreamOffsetEntry>(),
            size_of::<FileInformationPath>(),
            size_of::<FileInformationIndex>(),
            size_of::<SomeFolderThing>(),
            size_of::<Hash40>(),
            size_of::<DirectoryInfo>(),
            size_of::<DirectoryOffsets>(),
            size_of::<FolderHashIndex>(),
            size_of::<FileInfo2>(),
            size_of::<FileInfoSubIndex>(),
            size_of::<SubFileInfo>(),
            size_of::<StreamHashToName>(),
            size_of::<FileInformationUnknownTable>(),
            size_of::<HashIndexGroup>(),
        );
    }
}
