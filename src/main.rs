mod dependencies;
mod options;
mod search_path;
mod summarize;

use clap::Parser;
use object::Endianness;
use std::fs;
use term_table;
use term_table::row;

fn endian_as_str(end : Endianness) -> &'static str {
    match end {
        Endianness::Little => "little",
        Endianness::Big => "big"
    }
}

// fn format_dynamic_symbol_ref(sym_ref : &summarize::DynamicSymbolReference) -> String {
//     let mut s = String::from(&sym_ref.symbol.name);
//     match sym_ref.binding {
//         summarize::SymbolBinding::Global => {},
//         summarize::SymbolBinding::Weak => {
//             s.push_str(" (Weak)");
//         },
//         summarize::SymbolBinding::Local => {
//             s.push_str(" (Local)");
//         },
//         summarize::SymbolBinding::Unknown => {
//             s.push_str(" (Unknown)");
//         }
//     }

//     s
// }

fn render_dynamic_symbol_ref(sym_ref : &summarize::DynamicSymbolReference) -> Vec<String> {
    vec![format!("{:?}", sym_ref.type_), format!("{:?}", sym_ref.binding), String::from(&sym_ref.symbol.name)]
}

fn main() -> anyhow::Result<()> {
    let args = options::Options::parse();
    let summary = summarize::summarize_path(&args.input)?;
    let search_path = search_path::search_path(&args.sysroot, &summary);

    // TODO:
    //
    // - Resolve the libraries providing each external symbol

    println!("File {} is a {} bit {} endian ELF file",
             summary.filename.as_path().to_str().unwrap(),
             summary.bit_size,
             endian_as_str(summary.endianness));
    match &summary.binary_type {
        summarize::BinaryType::Static => {
            println!("  Static");
        },
        summarize::BinaryType::Dynamic(dyn_deps) => {
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

            println!("  Depends on dynamic symbols:");
            let mut sym_ref_table = term_table::Table::new();
            sym_ref_table.add_row(row::Row::new(vec!["Type", "Binding", "Symbol"]));
            for sym_ref in &dyn_deps.dynamic_symbol_refs {
                sym_ref_table.add_row(row::Row::new(render_dynamic_symbol_ref(&sym_ref)));
            }
            println!("{}", sym_ref_table.render());
        }
    }

    Ok(())
}
