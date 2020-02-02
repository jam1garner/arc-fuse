use super::{set_file, get_header, get_footer, FilePtr32, FileSlice};

#[repr(C)]
struct Header {
    magic: [u8; 4],
    section1: FilePtr32<Section1Header>,
    section2: FilePtr32<Section2Header>,
}

#[repr(C)]
struct Section1Header {
    // pointer relative to start of file
    array_offset: FilePtr32<u32>,
    unk: f32,
    len: u32,
    // Section1Header is immediately followed by u32 array
}

#[repr(C)]
struct Section2Header {
    // pointer relative to start of section 2
    rel_ptr: FilePtr32<f32>,
    // after the f32 is a u32
}

#[test]
fn parse_test_file() {
    let test_file = include_bytes!("test.bin");
    set_file(test_file);
    
    let header = get_header::<Header>();
    let footer: [u8; 4] = *get_footer();

    let section2 = &header.section2;
    let section2_offset = section2.inner();
    let rel_ptr = section2.rel_ptr.offset(section2_offset);
    let after_rel_ptr = rel_ptr.next::<u32>();

    let section1 = &header.section1;
    let len = section1.len as usize;
    let slice1 = section1.next_slice::<u32>(len);
    let slice2 = section1.array_offset.slice(len);
    
    assert_eq!(header.magic, *b"TEST");
    assert_eq!(footer, *b"ENDF");
    assert_eq!(header.section1.unk, 2.0);
    assert_eq!(rel_ptr, 1.0);
    assert_eq!(after_rel_ptr, 0x1337);
    assert_eq!(&*slice1, &[1u32, 2u32, 3u32]);
    assert_eq!(&*slice2, &[4u32, 5u32, 6u32]);
}
