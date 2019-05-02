use packed_struct::prelude::*;

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
    filesize: u32,
    file_info_count: u32,
    unk_offset_size_count: u32,
    folder_count: u32,

    file_count1: u32,

    hash_folder_count: u32,
    file_information_count: u32,
    last_table_count: u32,
    sub_file_count: u32,

    file_count2: u32,
    sub_file_count2: u32,
    unk11: u32,
    unk1_10: u32,
    unk2_10: u32,
    unk13: u32,
    unk14: u32,
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
