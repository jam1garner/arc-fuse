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
