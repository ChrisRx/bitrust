#![feature(file_set_times)]

extern crate scopeguard;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

mod corrupt;
use crate::corrupt::corrupt;

mod fix;
use crate::fix::fix;

mod scan;
use crate::scan::scan;

#[derive(StructOpt, Debug)]
#[structopt(name = "bitrust")]
enum BitRust {
    #[structopt(name = "corrupt")]
    Corrupt(CorruptOpts),

    #[structopt(name = "fix")]
    Fix(FixOpts),

    #[structopt(name = "scan")]
    Scan(ScanOpts),
}

#[derive(StructOpt, Debug)]
struct CorruptOpts {
    path: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
struct FixOpts {
    path: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
struct ScanOpts {
    #[structopt(short, long)]
    fatal: bool,

    path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env_logger::init_from_env(env_logger::Env::default().filter_or("RUST_LOG", "info"));

    match BitRust::from_args() {
        BitRust::Scan(opt) => {
            let path = opt.path.unwrap_or(env::current_dir()?);
            return scan(path, opt.fatal);
        }
        BitRust::Corrupt(opt) => {
            return corrupt(opt.path.unwrap());
        }
        BitRust::Fix(opt) => {
            return fix(opt.path.unwrap());
        }
    }
}
