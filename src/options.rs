use clap::Parser;
use std::path::PathBuf;

#[derive(Debug,Parser)]
#[clap(version = "1.0", author = "Tristan Ravitch")]
pub struct Options {
    #[clap(help="The file to examine")]
    pub input : PathBuf,
    #[clap(help="The system root to use to search for dependencies", long="sysroot", default_value="/")]
    pub sysroot : PathBuf
}
