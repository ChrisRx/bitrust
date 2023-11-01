use inquire::Confirm;
use rand::Rng;
use std::error::Error;
use std::fmt;
use std::fs::{FileTimes, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Debug)]
struct FileTooLargeError(PathBuf, u64);

impl fmt::Display for FileTooLargeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "file {} is {}, cannot corrupt a file bigger than 4kb",
            self.0.display(),
            self.1
        )
    }
}

impl Error for FileTooLargeError {}

pub fn corrupt(path: PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(path.clone())?;
    let atime = file.metadata()?.accessed()?;
    let mtime = file.metadata()?.modified()?;
    let mut b = [0u8; 1];
    let mut rng = rand::thread_rng();
    let len = file.metadata()?.len();
    if len > 4 * 1024 * 1024 {
        return Err(Box::new(FileTooLargeError(path.clone(), len)));
    }

    match Confirm::new(&format!("This will flip a random bit in the following file:\n\t{}\nAre you sure you want to proceed?", path.display()).to_string())
    .with_default(false)
    .prompt() {
        Ok(true) => {},
        Ok(false) => return Ok(()),
        Err(err) => return Err(err.into()),
    }
    let n = rng.gen_range(0..len);
    println!("n: {}", n);
    file.seek(SeekFrom::Start(n))?;
    file.read_exact(&mut b)?;
    println!("read 0b{:08b}", b[0]);
    b[0] ^= 0b0000_1000;
    println!("now 0b{:08b}", b[0]);
    file.seek(SeekFrom::Start(n))?;
    file.write(&b)?;
    file.set_times(FileTimes::new().set_modified(mtime).set_accessed(atime))?;
    Ok(())
}
