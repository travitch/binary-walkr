use clap::Parser;
use std::path::PathBuf;

#[derive(Debug,Parser)]
#[clap(version = "1.0", author = "Tristan Ravitch")]
pub struct Options {
    #[clap(help="The file to examine")]
    pub input : PathBuf
}
