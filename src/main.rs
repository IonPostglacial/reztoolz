use core::str;
use std::{borrow::Cow, env, fs::{create_dir, read, File}, io::Write, path::{Path, PathBuf}};

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
    Directory {
        children: RezDirectoryIterator<'a>,
    },
    File {
        id: u32,
        content: &'a [u8],
    }
}

struct RezEntry<'a> {
    path: PathBuf,
    datetime: u32,
    kind: Option<RezEntryContentKind<'a>>,
}

struct RezDirectoryIterator<'a> {
    input: &'a [u8],
    path: PathBuf,
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
            let name = String::from_utf8_lossy(&self.input[self.offset + 16..name_end]);
            self.offset = name_end + 1;
            self.end = name_end + 1 + entry_size;
            let path = self.path.join(&*name);
            Some(RezEntry {
                path: path.clone(), 
                datetime, 
                kind: Some(RezEntryContentKind::Directory {
                    children: RezDirectoryIterator {
                        input: &self.input,
                        path,
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
            let reversed_extension = String::from_utf8_lossy(&self.input[self.offset + 20..extension_end]);
            let mut name_end = extension_end + 5;
            while self.input[name_end] != b'\0' && name_end < self.end  {
                name_end += 1;
            }
            let mut name = str::from_utf8(&self.input[extension_end + 5..name_end]).expect("string to contain only ascii").to_string();
            name.push('.');
            name.extend(reversed_extension.chars().rev());
            let content = &self.input[entry_offset..entry_offset + entry_size];
            self.offset = name_end + 2;
            self.end = name_end + 2 + entry_size;
            Some(RezEntry {
                path: self.path.join(&*name), 
                datetime, 
                kind: Some(RezEntryContentKind::File { id: file_id, content }),
            })
        }
    }
}

fn display_rez_hierarchy(dir: &mut RezDirectoryIterator) {
    for entry in dir {
        match entry.kind {
            Some(RezEntryContentKind::Directory { mut children }) => {
                println!(">> dir: {}", entry.path.to_str().expect("path to be valid string"));
                display_rez_hierarchy(&mut children);
            }
            Some(RezEntryContentKind::File { id, content }) => {
                println!("- file: {}", entry.path.to_str().expect("path to be valid string"));
            }
            None => {}
        }   
    }
}

fn extract_rez_hierarchy(dir: &mut RezDirectoryIterator, output_dir: &Path) {
    for entry in dir {
        match entry.kind {
            Some(RezEntryContentKind::Directory { mut children }) => {
                create_dir(output_dir.join(entry.path)).expect("being able to create the directory");
                extract_rez_hierarchy(&mut children, output_dir);
            }
            Some(RezEntryContentKind::File { id, content }) => {
                let mut file = File::create(output_dir.join(entry.path)).expect("being able to create file");
                file.write(content).expect("being able to write in the file");
            }
            None => {}
        }   
    }
}

fn main() {
    let mut args = env::args();
    let input = args.nth(1).expect("input REZ file");
    let output = args.next().expect("output directory");
    let input = read(input).expect("file to exist");
    create_dir(&output).expect("being able to create the root directory");
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
    let mut root_iterator = RezDirectoryIterator {
        input: &input,
        path: Path::new("").to_path_buf(),
        offset: header.dir_offset,
        end: header.dir_offset + header.dir_size,
    };
    display_rez_hierarchy(&mut root_iterator);
    extract_rez_hierarchy(&mut root_iterator, Path::new(&output));
}
