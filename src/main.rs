use core::str;
use std::{borrow::Cow, fs::read, path::{Path, PathBuf}};

#[derive(Debug)]
struct RezHeader<'a> {
    description: Cow<'a, str>,
    version: u32,
    dir_offset: usize,
    dir_size: usize,
    datetime: u32,
    dir_name_max: usize,
    file_name_max: usize,
}

enum RezEntryContentKind<'a> {
    Directory,
    File {
        id: u32,
        content: &'a [u8],
    }
}

struct RezEntryContent<'a> {
    path: PathBuf,
    datetime: u32,
    kind: Option<RezEntryContentKind<'a>>,
}

struct RezEntry<'a> {
    content: RezEntryContent<'a>,
}

fn parse_entry<'a>(input: &'a [u8], offset: usize, end: usize, path: &Path, cb: fn (entry: RezEntry)) {
    if offset >= input.len() {
        return;
    }
    let is_directory = u32::from_le_bytes(input[offset..offset+4].try_into().unwrap()) == 1;
    let entry_offset = u32::from_le_bytes(input[offset+4..offset+8].try_into().unwrap()) as usize;
    let entry_size = u32::from_le_bytes(input[offset+8..offset+12].try_into().unwrap()) as usize;
    if entry_size == 0 {
        return;
    }
    let datetime = u32::from_le_bytes(input[offset+12..offset+16].try_into().unwrap());
    if is_directory {
        let mut name_end = offset+16;
        while input[name_end] != b'\0' && name_end < end  {
            name_end += 1;
        }
        let name = String::from_utf8_lossy(&input[offset+16..name_end]);
        cb(RezEntry {
            content: RezEntryContent { 
                path: path.join(&*name), 
                datetime, 
                kind: Some(RezEntryContentKind::Directory),
            }
        });
        if entry_size > 0 {
            parse_entry(input, entry_offset, entry_offset + entry_size, &path.join(&*name), cb);
        }
        parse_entry(input, name_end+1, name_end+1+entry_size, path, cb);
    } else {
        let file_id = u32::from_le_bytes(input[offset+16..offset+20].try_into().unwrap());
        let mut extension_end = offset+20;
        while input[extension_end] != b'\0' && extension_end < end {
            extension_end += 1;
        }
        let reversed_extension = String::from_utf8_lossy(&input[offset+20..extension_end]);
        let mut name_end = extension_end + 5;
        while input[name_end] != b'\0' && name_end < end  {
            name_end += 1;
        }
        let mut name = str::from_utf8(&input[extension_end+5..name_end]).expect("string to contain only ascii").to_string();
        name.push('.');
        name.extend(reversed_extension.chars().rev());
        let content = &input[entry_offset..entry_offset+entry_size];
        cb(RezEntry {
            content: RezEntryContent { 
                path: path.join(&*name), 
                datetime, 
                kind: Some(RezEntryContentKind::File { id: file_id, content }),
            }
        });
        parse_entry(input, name_end + 2, name_end + 2 + entry_size, path, cb); 
    }
    
}

fn main() {
    let input = read("GRUNTZ.REZ").expect("file to exist");
    let header = RezHeader {
        description: String::from_utf8_lossy(&input[0..127]),
        version: u32::from_le_bytes(input[127..131].try_into().unwrap()),
        dir_offset: u32::from_le_bytes(input[131..135].try_into().unwrap()) as usize,
        dir_size: u32::from_le_bytes(input[135..139].try_into().unwrap()) as usize,
        datetime: u32::from_le_bytes(input[147..151].try_into().unwrap()),
        dir_name_max: u32::from_le_bytes(input[155..159].try_into().unwrap()) as usize,
        file_name_max: u32::from_le_bytes(input[159..163].try_into().unwrap()) as usize,
    };
    println!("header: {header:#?}");
    parse_entry(&input, header.dir_offset, header.dir_offset + header.dir_size, Path::new(""), |entry: RezEntry| {
        match entry.content.kind {
            Some(RezEntryContentKind::Directory) => {
                println!(">> dir: {}", entry.content.path.to_str().expect("path to be valid string"))
            }
            Some(RezEntryContentKind::File { id, content }) => {
                println!("- file: {}", entry.content.path.to_str().expect("path to be valid string"));
            }
            None => {}
        }
    })
}
