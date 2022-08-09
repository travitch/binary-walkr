mod dependencies;
mod options;
mod resolve_symbols;
mod search_path;
mod summarize;
mod ui;

use clap::Parser;
use object::Endianness;
use std::collections;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use term_table::row;

fn endian_as_str(end : Endianness) -> &'static str {
    match end {
        Endianness::Little => "little",
        Endianness::Big => "big"
    }
}

fn render_dynamic_symbol_ref<'a>(resolutions : &collections::BTreeMap<summarize::VersionedSymbol, &'a summarize::ElfSummary>,
                                 sym_ref : &summarize::DynamicSymbolReference) -> Vec<String> {
    let provider = resolutions.get(&sym_ref.symbol).map_or(PathBuf::from("<Unresolved>"), |elf| elf.filename.clone());
    vec![format!("{:?}", sym_ref.type_), format!("{:?}", sym_ref.binding), String::from(&sym_ref.symbol.name), provider.to_string_lossy().into()]
}

fn render_defined_dynamic_symbol(sym_def : &summarize::ExportedDynamicSymbol) -> Vec<String> {
    vec![format!("{:#x}", sym_def.address),
         format!("{}", sym_def.size),
         format!("{:?}", sym_def.type_),
         format!("{:?}", sym_def.binding),
         String::from(&sym_def.symbol.name)]
}

fn render_summary(summary : &summarize::ElfSummary, deps : &collections::BTreeMap<String, Option<summarize::ElfSummary>>) -> anyhow::Result<()> {
    println!("File {} is a {} bit {} endian ELF file",
             summary.filename.as_path().to_str().unwrap(),
             summary.bit_size,
             endian_as_str(summary.endianness));
    match &summary.binary_type {
        summarize::BinaryType::Static => {
            println!("  Static");
        },
        summarize::BinaryType::Dynamic(dyn_deps) => {
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

            let all_libs = deps.values().filter_map(|x| x.as_ref()).collect();
            let symbol_resolutions = resolve_symbols::resolve_symbols(&dyn_deps.dynamic_symbol_refs, &all_libs);

            println!("  Depends on dynamic symbols:");
            let mut sym_ref_table = term_table::Table::new();
            sym_ref_table.add_row(row::Row::new(vec!["Type", "Binding", "Symbol", "Provider"]));
            for sym_ref in &dyn_deps.dynamic_symbol_refs {
                sym_ref_table.add_row(row::Row::new(render_dynamic_symbol_ref(&symbol_resolutions, sym_ref)));
            }
            println!("{}", sym_ref_table.render());

            if !dyn_deps.provided_dynamic_symbols.is_empty() {
                println!("  Defines dynamic symbols:");
                let mut sym_def_table = term_table::Table::new();
                sym_def_table.add_row(row::Row::new(vec!["Address", "Size", "Type", "Binding", "Symbol"]));
                for sym_def in &dyn_deps.provided_dynamic_symbols {
                    sym_def_table.add_row(row::Row::new(render_defined_dynamic_symbol(sym_def)));
                }

                println!("{}", sym_def_table.render());
            }
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = options::Options::parse();
    let summary = summarize::summarize_path(&args.input)?;
    let search_path = search_path::search_path(&args.sysroot, &summary);
    let deps = dependencies::resolve_dependencies(&search_path, &summary);


    if args.interactive {
        let dur = Duration::from_millis(250);
        return ui::crossterm::run(dur, &summary, &deps);
    }

    render_summary(&summary, &deps)?;
    Ok(())
}
