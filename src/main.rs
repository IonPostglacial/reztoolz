use core::str;
use std::{env, fs::{create_dir, read, File}, io::Write, path::Path};

#[derive(Debug)]
struct RezHeader<'a> {
    description: &'a str,
    version: u32,
    dir_offset: usize,
    dir_size: usize,
    datetime: u32,
    dir_name_max: usize,
    file_name_max: usize,
}

enum RezEntryContentKind<'a> {
    Directory {
        children: RezDirectoryIterator<'a>,
    },
    File {
        id: u32,
        extension: &'a [u8],
        content: &'a [u8],
    }
}

struct RezEntry<'a> {
    name: &'a [u8],
    datetime: u32,
    kind: Option<RezEntryContentKind<'a>>,
}

struct RezDirectoryIterator<'a> {
    input: &'a [u8],
    offset: usize,
    end: usize,
}

impl<'a> Iterator for RezDirectoryIterator<'a> {
    type Item = RezEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.input.len() {
            return None;
        }
        let is_directory = u32::from_le_bytes(self.input[self.offset..self.offset+4].try_into().unwrap()) == 1;
        let entry_offset = u32::from_le_bytes(self.input[self.offset+4..self.offset+8].try_into().unwrap()) as usize;
        let entry_size = u32::from_le_bytes(self.input[self.offset+8..self.offset+12].try_into().unwrap()) as usize;
        if entry_size == 0 {
            return None;
        }
        let datetime = u32::from_le_bytes(self.input[self.offset+12..self.offset+16].try_into().unwrap());
        if is_directory {
            let mut name_end = self.offset + 16;
            while self.input[name_end] != b'\0' && name_end < self.end  {
                name_end += 1;
            }
            let name = &self.input[self.offset + 16..name_end];
            self.offset = name_end + 1;
            self.end = name_end + 1 + entry_size;
            Some(RezEntry {
                name, 
                datetime, 
                kind: Some(RezEntryContentKind::Directory {
                    children: RezDirectoryIterator {
                        input: &self.input,
                        offset: entry_offset,
                        end: entry_offset + entry_size,
                    },
                }),
            })
        } else {
            let file_id = u32::from_le_bytes(self.input[self.offset + 16..self.offset + 20].try_into().unwrap());
            let mut extension_end = self.offset + 20;
            while self.input[extension_end] != b'\0' && extension_end < self.end {
                extension_end += 1;
            }
            let mut name_end = extension_end + 5;
            while self.input[name_end] != b'\0' && name_end < self.end  {
                name_end += 1;
            }
            let content = &self.input[entry_offset..entry_offset + entry_size];
            let name = &self.input[extension_end + 5..name_end];
            let extension = &self.input[self.offset + 20..extension_end];
            self.offset = name_end + 2;
            self.end = name_end + 2 + entry_size;
            Some(RezEntry {
                name, 
                datetime, 
                kind: Some(RezEntryContentKind::File { 
                    id: file_id,
                    extension,
                    content
                }),
            })
        }
    }
}

fn display_rez_hierarchy(dir: &mut RezDirectoryIterator, path: &Path) {
    for entry in dir {
        let name = str::from_utf8(entry.name).expect("path to be valid string");
        let full_path = path.join(Path::new(name));
        match entry.kind {
            Some(RezEntryContentKind::Directory { mut children }) => {
                println!(">> dir: {}", &full_path.to_str().expect("path to be valid string"));
                display_rez_hierarchy(&mut children, &full_path);
            }
            Some(RezEntryContentKind::File { id, extension, content }) => {
                let mut full_name = full_path.to_str().expect("path to be valid string").to_string();
                full_name.push('.');
                full_name.extend(str::from_utf8(extension).expect("extension to be valid string").chars().rev());
                println!("- file: {}", full_name);
            }
            None => {}
        }   
    }
}

fn extract_rez_hierarchy(dir: &mut RezDirectoryIterator, output_dir: &Path, path: &Path) {
    for entry in dir {
        let name = str::from_utf8(entry.name).expect("path to be valid string");
        let full_path = path.join(Path::new(name));
        match entry.kind {
            Some(RezEntryContentKind::Directory { mut children }) => {
                create_dir(output_dir.join(&full_path)).expect("being able to create the directory");
                extract_rez_hierarchy(&mut children, output_dir, &full_path);
            }
            Some(RezEntryContentKind::File { id, extension, content }) => {
                let mut full_name = full_path.to_str().expect("path to be valid string").to_string();
                full_name.push('.');
                full_name.extend(str::from_utf8(extension).expect("extension to be valid string").chars().rev());
                let mut file = File::create(output_dir.join(&full_name)).expect("being able to create file");
                file.write(content).expect("being able to write in the file");
            }
            None => {}
        }   
    }
}

fn main() {
    let mut args = env::args();
    let cmd = args.nth(1).expect("command name");
    let input = args.next().expect("input REZ file");
    let input = read(input).expect("file to exist");
    let header = RezHeader {
        description: str::from_utf8(&input[0..127]).expect("description to be valid utf-8"),
        version: u32::from_le_bytes(input[127..131].try_into().unwrap()),
        dir_offset: u32::from_le_bytes(input[131..135].try_into().unwrap()) as usize,
        dir_size: u32::from_le_bytes(input[135..139].try_into().unwrap()) as usize,
        datetime: u32::from_le_bytes(input[147..151].try_into().unwrap()),
        dir_name_max: u32::from_le_bytes(input[155..159].try_into().unwrap()) as usize,
        file_name_max: u32::from_le_bytes(input[159..163].try_into().unwrap()) as usize,
    };
    println!("header: {header:#?}");
    let mut root_iterator = RezDirectoryIterator {
        input: &input,
        offset: header.dir_offset,
        end: header.dir_offset + header.dir_size,
    };
    match cmd.as_str() {
        "tree" => display_rez_hierarchy(&mut root_iterator, Path::new("")),
        "extract" => {
            let output = args.next().expect("output directory");
            create_dir(&output).expect("being able to create the root directory");
            extract_rez_hierarchy(&mut root_iterator, Path::new(&output), Path::new(""));
        }
        _ => println!("unknown command {cmd}"),
    }
    
}
