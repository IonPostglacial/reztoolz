use core::str;
mod rez;
mod pid;
mod wwd;
use std::{env, fs::{create_dir, read, File}, io::Write, path::Path};

fn display_rez_hierarchy(dir: &mut rez::DirectoryIterator, path: &Path) {
    for entry in dir {
        let name = str::from_utf8(entry.name).expect("path to be valid string");
        let full_path = path.join(Path::new(name));
        match entry.kind {
            Some(rez::EntryContentKind::Directory { mut children }) => {
                println!(">> dir: {}", &full_path.to_str().expect("path to be valid string"));
                display_rez_hierarchy(&mut children, &full_path);
            }
            Some(rez::EntryContentKind::File { id, extension, content }) => {
                let mut full_name = full_path.to_str().expect("path to be valid string").to_string();
                full_name.push('.');
                full_name.extend(str::from_utf8(extension).expect("extension to be valid string").chars().rev());
                println!("- file: {}", full_name);
            }
            None => {}
        }   
    }
}

fn extract_rez_hierarchy(dir: &mut rez::DirectoryIterator, output_dir: &Path, path: &Path) {
    for entry in dir {
        let name = str::from_utf8(entry.name).expect("path to be valid string");
        let full_path = path.join(Path::new(name));
        match entry.kind {
            Some(rez::EntryContentKind::Directory { mut children }) => {
                create_dir(output_dir.join(&full_path)).expect("being able to create the directory");
                extract_rez_hierarchy(&mut children, output_dir, &full_path);
            }
            Some(rez::EntryContentKind::File { id, extension, content }) => {
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
    let mut archive = rez::Archive::parse(&input);
    match cmd.as_str() {
        "tree" => display_rez_hierarchy(&mut archive.root, Path::new("")),
        "extract" => {
            let output = args.next().expect("output directory");
            create_dir(&output).expect("being able to create the root directory");
            extract_rez_hierarchy(&mut archive.root, Path::new(&output), Path::new(""));
        }
        _ => println!("unknown command {cmd}"),
    }    
}
