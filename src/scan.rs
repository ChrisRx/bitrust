use bincode;
use log::{debug, info};
use rayon::prelude::*;
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};
use xxhash_rust::xxh3::Xxh3;

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub modified: SystemTime,
    pub hash: u64,
}

impl File {
    fn from_entry(entry: &DirEntry) -> Result<File, Box<dyn Error + Send + Sync>> {
        let file = fs::File::open(entry.path())?;
        let reader = BufReader::new(file);
        let hash = hash_file(reader)?;
        Ok(File {
            modified: entry.metadata()?.modified()?,
            hash,
        })
    }

    pub fn from_path(path: &PathBuf) -> Result<File, Box<dyn Error + Send + Sync>> {
        let file = fs::File::open(path)?;
        let modified = file.metadata()?.modified()?;
        let hash = hash_file(BufReader::new(file))?;
        Ok(File { modified, hash })
    }
}

const BUFFER_SIZE: usize = 4 * 1024;

pub fn hash_file<R: Read>(mut reader: R) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let mut buffer = [0_u8; BUFFER_SIZE];
    let mut hasher = Xxh3::new();

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.digest())
}

pub fn scan(path: PathBuf, fatal: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let tree = sled::open("data")?;
    defer! {
        tree.flush().unwrap();
    }

    WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .try_for_each_with(&tree, |t, e| -> Result<(), Box<dyn Error + Send + Sync>> {
            if let Err(err) = scan_entry(t, e) {
                if fatal {
                    return Err(From::from(err));
                }
                println!("whoopsie doopsie: {:?}", err);
                return Ok(());
            }
            Ok(())
        })?;

    Ok(())
}

fn scan_entry(
    tree: &sled::Db,
    entry: Result<walkdir::DirEntry, walkdir::Error>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let entry = entry?;
    if !entry.file_type().is_file() {
        return Ok(());
    }
    let new_file = File::from_entry(&entry)?;
    let key = entry.path().to_str().unwrap();

    debug!("key: {}, hash: {}", key, new_file.hash);
    match tree.get(key)? {
        Some(bytes) => {
            let old_file: File = bincode::deserialize(&bytes)?;
            debug!("old: {old_file:?}");
            debug!("new: {new_file:?}");
            if new_file.hash != old_file.hash {
                if new_file.modified > old_file.modified {
                    tree.insert(key, bincode::serialize(&new_file)?)?;
                } else {
                    info!("corrupted!");
                }
            }
        }
        None => {
            tree.insert(key, bincode::serialize(&new_file)?)?;
        }
    }
    Ok(())
}
