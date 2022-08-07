mod dependencies;
mod options;
mod search_path;
mod summarize;

use clap::Parser;
use object::Endianness;
use std::fs;

fn endian_as_str(end : Endianness) -> &'static str {
    match end {
        Endianness::Little => "little",
        Endianness::Big => "big"
    }
}


fn main() -> anyhow::Result<()> {
    let args = options::Options::parse();
    let summary = summarize::summarize_path(&args.input)?;
    let search_path = search_path::search_path(&args.sysroot, &summary);

    println!("File {} is a {} bit {} endian ELF file",
             summary.filename.as_path().to_str().unwrap(),
             summary.bit_size,
             endian_as_str(summary.endianness));
    match &summary.dependencies {
        summarize::BinaryDependencies::Static => {
            println!("  Static");
        },
        summarize::BinaryDependencies::Dynamic(_dyn_deps) => {
            let deps = dependencies::resolve_dependencies(&search_path, &summary);
            println!("  Dynamically linked against:");

            for (dep_name, dep_summary) in deps {
                match dep_summary {
                    None => {
                        println!("    {} -> Unresolved", dep_name)
                    },
                    Some(dep_summary) => {
                        // Resolve symbolic links before display
                        let disp_path = fs::canonicalize(dep_summary.filename.as_path())?;
                        println!("    {} -> {}", dep_name, disp_path.as_path().to_string_lossy());
                    }
                }
            }
        }
    }

    Ok(())
}
