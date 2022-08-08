use object::Endianness;
use object::read::{SectionIndex, StringTable};
use object::read::elf as elf_reader;
use object::read::elf::{FileHeader, Dyn, Sym};
use object::elf;
use std::fs;
use std::path::{PathBuf};

#[derive(thiserror::Error,Debug)]
pub enum WalkError {
    #[error("Missing expected `.dynstr` section")]
    MissingExpectedDynstrSection,
    #[error("Missing expected `.dynsym` section")]
    MissingExpectedDynsymSection
}

/// A (possibly) versioned symbol
#[derive(Eq, Ord, PartialOrd, PartialEq, Clone)]
pub struct VersionedSymbol {
    pub name : String,
    pub version : Option<String>
}

impl VersionedSymbol {
    fn new<Elf : FileHeader>(end : Elf::Endian, dyn_strings : &StringTable, sym : &Elf::Sym) -> Self {
        let sym_name = sym.name(end, *dyn_strings).map_or(String::from("<Error>"), |bytes| String::from_utf8_lossy(bytes).into_owned());
        // FIXME: Symbol versions are actually in another section - need to look them up
        VersionedSymbol {
            name : sym_name,
            version : None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SymbolType {
    Func,
    Object,
    Common,
    NoType,
    File,
    Unknown
}

impl SymbolType {
    fn new(ty : u8) -> Self {
        match ty {
            elf::STT_FUNC => SymbolType::Func,
            elf::STT_OBJECT => SymbolType::Object,
            elf::STT_COMMON => SymbolType::Common,
            elf::STT_NOTYPE => SymbolType::NoType,
            elf::STT_FILE => SymbolType::File,
            _ => SymbolType::Unknown
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
    Unknown
}

impl SymbolBinding {
    fn new(b : u8) -> Self {
        match b {
            elf::STB_LOCAL => SymbolBinding::Local,
            elf::STB_WEAK => SymbolBinding::Weak,
            elf::STB_GLOBAL => SymbolBinding::Global,
            _ => SymbolBinding::Unknown
        }
    }
}

/// A reference to an external dynamic symbol in a binary
pub struct DynamicSymbolReference {
    pub symbol : VersionedSymbol,
    pub type_ : SymbolType,
    pub binding : SymbolBinding
}

/// A dynamic symbol provided by this binary
pub struct ExportedDynamicSymbol {
    pub symbol : VersionedSymbol,
    pub type_ : SymbolType,
    pub binding : SymbolBinding,
    pub size : u64,
    pub address : u64
}

/// Information summarizing the interface of a dynamically-linked binary or library
pub struct DynamicData {
    /// Dynamic symbols that this binary references
    pub dynamic_symbol_refs : Vec<DynamicSymbolReference>,
    /// Dynamic symbols provided by this binary
    pub provided_dynamic_symbols : Vec<ExportedDynamicSymbol>,
    /// The names of libraries that this binary pulls in as dynamic dependencies
    pub deps : Vec<String>
}

pub enum BinaryType {
    Static,
    Dynamic(DynamicData)
}

// TODO:
//
// - Include the section list
// - Include the segment mapping
//
// - Render the layout
//
// - Add interactive mode

pub struct ElfSummary {
    pub endianness : Endianness,
    pub bit_size : usize,
    pub filename : PathBuf,
    pub binary_type : BinaryType
}

fn analyze_dependencies<Elf>(bytes : &[u8], obj : &Elf, sec_table : &elf_reader::SectionTable<Elf>) -> anyhow::Result<BinaryType>
where Elf : elf_reader::FileHeader<Endian = Endianness> {
    let end = obj.endian()?;
    match sec_table.dynamic(end, bytes)? {
        None => Ok(BinaryType::Static),
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

            let mut undef_symbols = Vec::new();
            let mut def_symbols = Vec::new();
            let (dynsym_sec_idx, _dynsym_sec) = sec_table.section_by_name(end, ".dynsym".as_bytes()).ok_or(WalkError::MissingExpectedDynsymSection)?;
            let dyn_symtab = sec_table.symbol_table_by_index(end, bytes, SectionIndex(dynsym_sec_idx))?;
            for sym in dyn_symtab.symbols() {
                let sym_name = VersionedSymbol::new::<Elf>(end, &dyn_strings, &sym);
                if sym_name.name.len() == 0 {
                    continue;
                }

                if sym.is_undefined(end) {
                    let dyn_ref = DynamicSymbolReference {
                        symbol : sym_name,
                        type_ : SymbolType::new(sym.st_type()),
                        binding : SymbolBinding::new(sym.st_bind())
                    };
                    undef_symbols.push(dyn_ref);
                } else {
                    let dyn_ref = ExportedDynamicSymbol {
                        symbol : sym_name,
                        type_ : SymbolType::new(sym.st_type()),
                        binding : SymbolBinding::new(sym.st_bind()),
                        size : sym.st_size(end).into(),
                        address : sym.st_value(end).into()
                    };
                    def_symbols.push(dyn_ref);
                }
            }

            let dyn_data = DynamicData {
                deps : deps,
                dynamic_symbol_refs : undef_symbols,
                provided_dynamic_symbols : def_symbols
            };
            Ok(BinaryType::Dynamic(dyn_data))
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
        binary_type : deps
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
