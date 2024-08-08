use core::str;

#[derive(Debug)]
pub struct Header<'a> {
    pub description: &'a str,
    pub version: u32,
    pub dir_offset: usize,
    pub dir_size: usize,
    pub datetime: u32,
    pub dir_name_max: usize,
    pub file_name_max: usize,
}

pub enum EntryContentKind<'a> {
    Directory {
        children: DirectoryIterator<'a>,
    },
    File {
        id: u32,
        extension: &'a [u8],
        content: &'a [u8],
    }
}

pub struct Entry<'a> {
    pub name: &'a [u8],
    pub datetime: u32,
    pub kind: Option<EntryContentKind<'a>>,
}

pub struct DirectoryIterator<'a> {
    pub input: &'a [u8],
    pub offset: usize,
    pub end: usize,
}

impl<'a> Iterator for DirectoryIterator<'a> {
    type Item = Entry<'a>;

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
            Some(Entry {
                name, 
                datetime, 
                kind: Some(EntryContentKind::Directory {
                    children: DirectoryIterator {
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
            Some(Entry {
                name, 
                datetime, 
                kind: Some(EntryContentKind::File { 
                    id: file_id,
                    extension,
                    content
                }),
            })
        }
    }
}

pub struct Archive<'a> {
    pub header: Header<'a>,
    pub root: DirectoryIterator<'a>,
}

impl<'a> Archive<'a> {
    pub fn parse(input: &'a [u8]) -> Archive<'a> {
        let header = Header {
            description: str::from_utf8(&input[0..127]).expect("description to be valid utf-8"),
            version: u32::from_le_bytes(input[127..131].try_into().unwrap()),
            dir_offset: u32::from_le_bytes(input[131..135].try_into().unwrap()) as usize,
            dir_size: u32::from_le_bytes(input[135..139].try_into().unwrap()) as usize,
            datetime: u32::from_le_bytes(input[147..151].try_into().unwrap()),
            dir_name_max: u32::from_le_bytes(input[155..159].try_into().unwrap()) as usize,
            file_name_max: u32::from_le_bytes(input[159..163].try_into().unwrap()) as usize,
        };
        let root = DirectoryIterator {
            input: &input,
            offset: header.dir_offset,
            end: header.dir_offset + header.dir_size,
        };
        Archive { header, root }
    }
}