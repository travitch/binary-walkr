use std::collections;

use crate::summarize::{ElfSummary, VersionedSymbol, DynamicSymbolReference, BinaryType};

/// Determine which dependencies provide each dynamic symbol referenced by the given `ElfSummary`
pub fn resolve_symbols<'a>(dyn_sym_refs : &Vec<DynamicSymbolReference>, deps : &Vec<&'a ElfSummary>) ->
    collections::BTreeMap<VersionedSymbol, &'a ElfSummary>
{
    let mut res = collections::BTreeMap::new();
    let mut needed_syms = collections::HashSet::new();

    for dyn_sym in dyn_sym_refs {
        needed_syms.insert(&dyn_sym.symbol.name);
    }

    for dep in deps {
        match &dep.binary_type {
            BinaryType::Static => {},
            BinaryType::Dynamic(dyn_data) => {
                for defined_sym in &dyn_data.provided_dynamic_symbols {
                    match needed_syms.get(&defined_sym.symbol.name) {
                        None => {},
                        Some(_) => {
                            res.insert(defined_sym.symbol.clone(), *dep);
                        }
                    }
                }
            }
        }
    }

    res
}
