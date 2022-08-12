use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections;
use std::path::PathBuf;
use tui::widgets::{ListState, TableState};

use crate::resolve_symbols::resolve_symbols;
use crate::summarize;

#[derive(Copy, Clone)]
pub enum InfoTabLabels {
    Overview,
    DynamicDependencies,
    DefinedDynamicSymbols,
}

impl std::fmt::Display for InfoTabLabels {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InfoTabLabels::Overview => {
                write!(f, "Overview")
            }
            InfoTabLabels::DynamicDependencies => {
                write!(f, "Dynamic Dependencies")
            }
            InfoTabLabels::DefinedDynamicSymbols => {
                write!(f, "Defined Dynamic Symbols")
            }
        }
    }
}

/// The state of the tab bar for a *single* binary
///
/// Each binary has its own state
///
/// The labels are included in the tab state, as different types of binary will soon have different tabs
pub struct TabState {
    pub tab_labels: Vec<InfoTabLabels>,
    pub selected_tab: usize,
}

impl TabState {
    fn new() -> Self {
        TabState {
            tab_labels: vec![
                InfoTabLabels::Overview,
                InfoTabLabels::DynamicDependencies,
                InfoTabLabels::DefinedDynamicSymbols,
            ],
            selected_tab: 0,
        }
    }

    fn selected_label(&self) -> InfoTabLabels {
        self.tab_labels[self.selected_tab]
    }
}

fn increment_table_selection(table_state: &mut TableState, num_items: usize) {
    if num_items == 0 {
        return;
    }

    match table_state.selected() {
        None => {
            table_state.select(Some(0));
        }
        Some(cur_idx) => {
            table_state.select(Some(std::cmp::min(num_items - 1, cur_idx + 1)));
        }
    }
}

fn decrement_table_selection(table_state: &mut TableState, num_items: usize) {
    if num_items == 0 {
        return;
    }

    match table_state.selected() {
        None => {
            table_state.select(Some(num_items - 1));
        }
        Some(0) => {}
        Some(cur_idx) => {
            table_state.select(Some(std::cmp::max(0, cur_idx - 1)));
        }
    }
}

pub struct BinaryUIState {
    pub tab_state: TabState,
    pub defined_dynamic_table_state: TableState,
    pub dynamic_reference_table_state: TableState,
}

impl BinaryUIState {
    fn new() -> Self {
        BinaryUIState {
            tab_state: TabState::new(),
            defined_dynamic_table_state: TableState::default(),
            dynamic_reference_table_state: TableState::default(),
        }
    }
}

pub struct StaticAppData<'a> {
    pub title: String,
    pub elf: &'a summarize::ElfSummary,
    pub resolved_dependencies: &'a collections::BTreeMap<String, Option<summarize::ElfSummary>>,
    pub symbol_resolutions:
        collections::BTreeMap<summarize::VersionedSymbol, &'a summarize::ElfSummary>,
}

pub struct MutableAppData {
    pub should_quit: bool,
    pub selected_binary: ListState,
    /// The state of the tab widget for each binary
    ///
    /// This is initialized on demand
    pub binary_ui_state: collections::BTreeMap<PathBuf, BinaryUIState>,
}

impl MutableAppData {
    /// Get the tab state for the given binary; note that this is total because
    /// it will lazily instantiate tab state if needed.
    ///
    /// It takes the binary state is being requested for as evidence that a
    /// binary is selected; it does not rely on the binary selection in
    /// `selected_binary`.
    pub fn binary_ui_state(&mut self, bin: &summarize::ElfSummary) -> &mut BinaryUIState {
        self.binary_ui_state
            .entry(bin.filename.clone())
            .or_insert_with(BinaryUIState::new)
    }
}

/// Application state
pub struct App<'a> {
    pub static_app_data: StaticAppData<'a>,
    pub mutable_app_data: MutableAppData,
}

impl<'a> App<'a> {
    pub fn new(
        title: &str,
        elf_summary: &'a summarize::ElfSummary,
        resolved_deps: &'a collections::BTreeMap<String, Option<summarize::ElfSummary>>,
    ) -> Self {
        let all_libs = resolved_deps.values().filter_map(|x| x.as_ref()).collect();
        let mut resolved_syms = match &elf_summary.binary_type {
            summarize::BinaryType::Static => collections::BTreeMap::new(),
            summarize::BinaryType::Dynamic(dyn_data) => {
                resolve_symbols(&dyn_data.dynamic_symbol_refs, &all_libs)
            }
        };
        for lib in &all_libs {
            match &lib.binary_type {
                summarize::BinaryType::Static => {}
                summarize::BinaryType::Dynamic(dyn_data) => {
                    let mut lib_resolutions =
                        resolve_symbols(&dyn_data.dynamic_symbol_refs, &all_libs);
                    resolved_syms.append(&mut lib_resolutions);
                }
            }
        }

        let static_data = StaticAppData {
            title: title.to_string(),
            elf: elf_summary,
            resolved_dependencies: resolved_deps,
            symbol_resolutions: resolved_syms,
        };

        let mutable_data = MutableAppData {
            should_quit: false,
            selected_binary: ListState::default(),
            binary_ui_state: collections::BTreeMap::new(),
        };

        App {
            static_app_data: static_data,
            mutable_app_data: mutable_data,
        }
    }

    pub fn selected_binary(&self) -> Option<&'a summarize::ElfSummary> {
        match self.mutable_app_data.selected_binary.selected() {
            None => None,
            Some(idx) => {
                if idx == 0 {
                    Some(self.static_app_data.elf)
                } else {
                    let v = self
                        .static_app_data
                        .resolved_dependencies
                        .values()
                        .map(|o| o.as_ref())
                        .collect::<Vec<Option<&summarize::ElfSummary>>>();
                    v[idx - 1]
                }
            }
        }
    }

    pub fn on_key(&mut self, evt: KeyEvent) {
        match evt.code {
            KeyCode::Char('q') if evt.modifiers == KeyModifiers::CONTROL => {
                self.mutable_app_data.should_quit = true;
            }
            KeyCode::Char('p') if evt.modifiers == KeyModifiers::CONTROL => {
                let num_bins = 1 + self.static_app_data.resolved_dependencies.len();
                match self.mutable_app_data.selected_binary.selected() {
                    None => {
                        self.mutable_app_data
                            .selected_binary
                            .select(Some(num_bins - 1));
                    }
                    Some(sel_idx) if sel_idx == 0 => {
                        // No-op
                    }
                    Some(sel_idx) => {
                        self.mutable_app_data
                            .selected_binary
                            .select(Some(sel_idx - 1));
                    }
                }
            }
            KeyCode::Char('n') if evt.modifiers == KeyModifiers::CONTROL => {
                let num_bins = 1 + self.static_app_data.resolved_dependencies.len();
                match self.mutable_app_data.selected_binary.selected() {
                    None => {
                        self.mutable_app_data.selected_binary.select(Some(0));
                    }
                    Some(sel_idx) => {
                        self.mutable_app_data
                            .selected_binary
                            .select(Some(std::cmp::min(sel_idx + 1, num_bins - 1)));
                    }
                }
            }
            KeyCode::Char(c)
                if evt.modifiers == KeyModifiers::ALT
                    && c >= '1'
                    && c <= '9' =>
            {
                // c.is_ascii_digit()
                // The user wants to switch info pane using ALT+#
                match self.selected_binary() {
                    None => {}
                    Some(bin) => {
                        // If we are here at all, the tab state has been instantiated
                        match self.mutable_app_data.binary_ui_state.get_mut(&bin.filename) {
                            None => {}
                            Some(ui_state) => {
                                // We have already ensured that this character is a digit
                                let mut user_req = c.to_digit(10).unwrap() as usize;
                                user_req -= 1;
                                if user_req < ui_state.tab_state.tab_labels.len() {
                                    ui_state.tab_state.selected_tab = user_req;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Up => {
                match self.selected_binary() {
                    None => {}
                    Some(elf_summ) => match &elf_summ.binary_type {
                        summarize::BinaryType::Static => {}
                        summarize::BinaryType::Dynamic(dyn_data) => {
                            let ui_state = self.mutable_app_data.binary_ui_state(elf_summ);
                            let selected_tab = ui_state.tab_state.selected_label();
                            match selected_tab {
                                InfoTabLabels::DefinedDynamicSymbols => {
                                    let num_items = dyn_data.provided_dynamic_symbols.len();
                                    decrement_table_selection(
                                        &mut ui_state.defined_dynamic_table_state,
                                        num_items,
                                    );
                                }
                                InfoTabLabels::DynamicDependencies => {
                                    let num_items = dyn_data.dynamic_symbol_refs.len();
                                    decrement_table_selection(
                                        &mut ui_state.dynamic_reference_table_state,
                                        num_items,
                                    );
                                }
                                InfoTabLabels::Overview => {}
                            }
                        }
                    },
                }
            }
            KeyCode::Down => {
                match self.selected_binary() {
                    None => {}
                    Some(elf_summ) => match &elf_summ.binary_type {
                        summarize::BinaryType::Static => {}
                        summarize::BinaryType::Dynamic(dyn_data) => {
                            let ui_state = self.mutable_app_data.binary_ui_state(elf_summ);
                            let selected_tab = ui_state.tab_state.selected_label();
                            match selected_tab {
                                InfoTabLabels::Overview => {}
                                InfoTabLabels::DefinedDynamicSymbols => {
                                    let num_items = dyn_data.provided_dynamic_symbols.len();
                                    increment_table_selection(
                                        &mut ui_state.defined_dynamic_table_state,
                                        num_items,
                                    );
                                }
                                InfoTabLabels::DynamicDependencies => {
                                    let num_items = dyn_data.dynamic_symbol_refs.len();
                                    increment_table_selection(
                                        &mut ui_state.dynamic_reference_table_state,
                                        num_items,
                                    );
                                }
                            }
                        }
                    },
                }
            }
            _ => {}
        }
    }
}
