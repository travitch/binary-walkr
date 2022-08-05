use std::env;
use std::path::{PathBuf};

/// Compute the shared library search path based on system defaults and `LD_LIBRARY_PATH`
///
/// This does not yet consult the top-level summary to find DT_RUNPATH, but it needs to
pub fn search_path(_summ : &crate::summarize::ElfSummary) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    match env::var("LD_LIBRARY_PATH") {
        Err(_) => {},
        Ok(path_str) => {
            for path in env::split_paths(&path_str) {
                paths.push(PathBuf::from(path));
            }
        }
    }

    // The default paths used by the dynamic loader; note that this could vary
    // somewhat by system, so this list may need to be expanded
    paths.push(PathBuf::from("/lib"));
    paths.push(PathBuf::from("/lib64"));
    paths.push(PathBuf::from("/usr/lib"));
    paths.push(PathBuf::from("/usr/lib64"));

    paths
}

/* Note [Search Path]

1. Paths specified via DT_RPATH (deprecated, applies to *all* binary modules)
2. Paths in LD_LIBRARY_PATH
3. Paths in DT_RUNPATH (note: only applies to dependencies of the binary being looked up)
4. Default paths

*/
