use std::collections;
use std::path::PathBuf;

use crate::summarize::{summarize_path, BinaryType, ElfSummary};

#[derive(thiserror::Error, Debug)]
pub enum SearchError {
    #[error("Could not find dependency {0}")]
    MissingLibraryDependency(String),
}

fn analyze_one_dependency(
    search_path: &Vec<PathBuf>,
    lib_name: &str,
) -> anyhow::Result<ElfSummary> {
    for dir in search_path {
        let candidate = dir.join(PathBuf::from(lib_name));
        match summarize_path(&candidate) {
            Err(_) => {}
            Ok(summ) => {
                return Ok(summ);
            }
        }
    }
    Err(anyhow::Error::new(SearchError::MissingLibraryDependency(
        lib_name.to_string(),
    )))
}

struct WorkQueue {
    work_items: collections::VecDeque<String>,
    seen_items: collections::HashSet<String>,
}

impl WorkQueue {
    fn new() -> Self {
        WorkQueue {
            work_items: collections::VecDeque::new(),
            seen_items: collections::HashSet::new(),
        }
    }

    fn add_dependencies(&mut self, summ: &ElfSummary) {
        match &summ.binary_type {
            BinaryType::Static => {}
            BinaryType::Dynamic(dyn_deps) => {
                for dep in &dyn_deps.deps {
                    match self.seen_items.get(dep.as_str()) {
                        None => {
                            self.work_items.push_back(dep.to_string());
                            self.seen_items.insert(dep.to_string());
                        }
                        Some(_) => {}
                    }
                }
            }
        }
    }

    fn take_work(&mut self) -> Option<String> {
        self.work_items.pop_front()
    }
}

/// Recursively search for dependencies on the search path
///
/// The Elf summaries will not include the input binary
pub fn resolve_dependencies(
    search_path: &Vec<PathBuf>,
    summ: &ElfSummary,
) -> collections::BTreeMap<String, Option<ElfSummary>> {
    let mut res = collections::BTreeMap::new();
    let mut queue = WorkQueue::new();

    queue.add_dependencies(summ);

    while let Some(dep_name) = queue.take_work() {
        match analyze_one_dependency(search_path, dep_name.as_str()) {
            Err(_) => {
                // Report this as a failed lookup
                res.insert(dep_name, None);
            }
            Ok(dep_summary) => {
                queue.add_dependencies(&dep_summary);
                res.insert(dep_name, Some(dep_summary));
            }
        }
    }

    res
}
