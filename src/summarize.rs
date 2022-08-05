use object::Endianness;
use object::read::SectionIndex;
use object::read::elf as elf_reader;
use object::read::elf::{FileHeader, Dyn};
use object::elf;
use std::fs;
use std::path::{PathBuf};

#[derive(thiserror::Error,Debug)]
pub enum WalkError {
    #[error("Missing expected `.dynstr` section")]
    MissingExpectedDynstrSection
}

pub struct DynamicDependencies {
    /// The names of libraries that this binary pulls in as dynamic dependencies
    pub deps : Vec<String>
}

pub enum BinaryDependencies {
    Static,
    Dynamic(DynamicDependencies)
}

pub struct ElfSummary {
    pub endianness : Endianness,
    pub bit_size : usize,
    pub filename : PathBuf,
    pub dependencies : BinaryDependencies
}

fn analyze_dependencies<Elf>(bytes : &[u8], obj : &Elf, sec_table : &elf_reader::SectionTable<Elf>) -> anyhow::Result<BinaryDependencies>
where Elf : elf_reader::FileHeader<Endian = Endianness> {
    let end = obj.endian()?;
    match sec_table.dynamic(end, bytes)? {
        None => Ok(BinaryDependencies::Static),
        Some((dyn_entries, _dyn_idx)) => {
            // We need strings from the dynamic string table (.strtab is for
            // *static* strings that hold symbol strings, which are not relevant
            // for resolving dynamic strings).
            let (string_sec_idx, _string_sec) = sec_table.section_by_name(end, ".dynstr".as_bytes()).ok_or(WalkError::MissingExpectedDynstrSection)?;
            let dyn_strings = sec_table.strings(end, bytes, SectionIndex(string_sec_idx))?;
            let mut deps = Vec::new();

            for d in dyn_entries {
                match d.tag32(end) {
                    None => {},
                    Some(tag) => {
                        if tag == elf::DT_NEEDED {
                            let needed_string_bytes = d.string(end, dyn_strings)?;
                            let needed_string = String::from_utf8(needed_string_bytes.to_vec())?;
                            deps.push(needed_string.clone());
                        }
                    }
                }
            }

            let dyn_deps = DynamicDependencies {
                deps : deps
            };
            Ok(BinaryDependencies::Dynamic(dyn_deps))
        }
    }
}

fn summarize_elf<Elf : elf_reader::FileHeader<Endian = Endianness>>(f : &PathBuf, bytes : &[u8], obj : &Elf) -> anyhow::Result<ElfSummary> {
    let end = obj.endian()?;
    let sec_table = obj.sections(end, bytes)?;

    let deps = analyze_dependencies(bytes, obj, &sec_table)?;
    let bs = ElfSummary {
        endianness : if obj.is_little_endian() { Endianness::Little } else { Endianness::Big },
        bit_size : if obj.is_class_32() { 32 } else { 64 },
        filename : f.clone(),
        dependencies : deps
    };
    Ok(bs)
}

pub fn summarize_path(path : &PathBuf) -> anyhow::Result<ElfSummary> {
    let bytes = fs::read(path)?;
    match elf::FileHeader64::<Endianness>::parse(bytes.as_slice()) {
        Ok(e64) => summarize_elf(path, bytes.as_slice(), e64),
        Err(_) => {
            match elf::FileHeader32::<Endianness>::parse(bytes.as_slice()) {
                Ok(e32) => summarize_elf(path, bytes.as_slice(), e32),
                Err(_) => unimplemented!()
            }
        }
    }
}
