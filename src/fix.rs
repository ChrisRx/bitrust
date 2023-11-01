use bincode;
use log::info;
use scopeguard::defer;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::scan::hash_file;
use crate::scan::File;

pub fn fix(path: PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    let tree = sled::open("data")?;
    defer! {
        tree.flush().unwrap();
    }

    let new_file = File::from_path(&path)?;
    let key = path.to_str().unwrap();

    match tree.get(key)? {
        Some(bytes) => {
            let old_file: File = bincode::deserialize(&bytes)?;
            if new_file.hash != old_file.hash {
                info!("corrupted! let's fix!");
                let f = fs::File::open(path.clone())?;
                let len = f.metadata()?.len();
                let mut t = Twiddler {
                    file: BufReader::new(f),
                    pos: 0,
                    offset: 0,
                    bit: 0,
                };
                for offset in 0..len {
                    for bit in 0..8 {
                        t.twiddle(offset, bit)?;
                        let h = hash_file(&mut t)?;
                        if h == old_file.hash {
                            let mut file = OpenOptions::new()
                                .read(true)
                                .write(true)
                                .create(false)
                                .open(path.clone())?;
                            let mut b = [0u8; 1];
                            file.seek(SeekFrom::Start(offset))?;
                            file.read_exact(&mut b)?;
                            b[0] ^= 1 << bit;
                            file.seek(SeekFrom::Start(offset))?;
                            file.write(&b)?;

                            println!("h({offset:}, {bit}): {h:?}");
                        }
                    }
                }
            } else {
                info!("not corrupted");
            }
        }
        None => return Err("idk man".into()),
    }
    Ok(())
}

struct Twiddler {
    file: BufReader<fs::File>,
    pos: u64,
    offset: u64,
    bit: i32,
}

impl Twiddler {
    fn reset(&mut self) -> Result<u64, std::io::Error> {
        self.seek(SeekFrom::Start(0))
    }
    fn twiddle(&mut self, offset: u64, bit: i32) -> Result<u64, std::io::Error> {
        self.offset = offset;
        self.bit = bit;
        self.reset()
    }
}

impl Read for Twiddler {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.file.read(buf)?;
        if self.pos <= self.offset && self.offset < (self.pos + buf.len() as u64) {
            buf[(self.offset - self.pos) as usize] ^= 1 << self.bit;
        }
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Twiddler {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let n = self.file.seek(pos)?;
        self.pos = n;
        Ok(n)
    }
}
