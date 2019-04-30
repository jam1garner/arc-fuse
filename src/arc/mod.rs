#![allow(dead_code)]
use std::fs::File;
use std::path::Path;
use std::io;
use std::mem;

mod util;
mod structs;
mod parser;
use structs::*;
use parser::*;
use memmap::Mmap;

pub struct Arc {
    file: File,
    map: Mmap,
    decomp_table: Vec<u8>,
}

impl<'a> Arc {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let file = File::open(path.as_ref())?;
        let map = unsafe { Mmap::map(&file) }?;

        let mut arc = Arc {
            file,
            map,
            decomp_table: vec![],
        };
        dbg!(&arc.map[0..0x10]);
        dbg!(arc.header());
        arc.decompress_table();

        Ok(arc)
    }

    fn read_from_offset<T>(&self, offset: usize) -> &'a T {
        unsafe {
            mem::transmute(self.map.as_ptr().offset(offset as isize))
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
}
