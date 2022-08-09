use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections;
use std::path::PathBuf;
use tui::widgets::{ListState, TableState};

use crate::summarize;

#[derive(Copy, Clone)]
pub enum InfoTabLabels {
    Overview,
    DynamicDependencies,
    DefinedDynamicSymbols
}

impl std::fmt::Display for InfoTabLabels {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InfoTabLabels::Overview => {
                write!(f, "Overview")
            },
            InfoTabLabels::DynamicDependencies => {
                write!(f, "Dynamic Dependencies")
            },
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
#[derive(Clone)]
pub struct TabState {
    pub tab_labels : Vec<InfoTabLabels>,
    pub selected_tab : usize
}

impl TabState {
    fn new() -> Self {
        TabState {
            tab_labels : vec![InfoTabLabels::Overview, InfoTabLabels::DynamicDependencies, InfoTabLabels::DefinedDynamicSymbols],
            selected_tab : 0
        }
    }
}

#[derive(Clone)]
pub struct BinaryUIState {
    pub tab_state : TabState,
    pub defined_dynamic_table_state : TableState
}

impl BinaryUIState {
    fn new() -> Self {
        BinaryUIState {
            tab_state : TabState::new(),
            defined_dynamic_table_state : TableState::default()
        }
    }
}

/// The focused component of the UI (i.e., the component receiving keystrokes that are not global)
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub enum Focus {
    /// Focus is on the sidebar; note that the selected item is stored in the
    /// `selected_binary` `ListState`, as we don't want to lose that state when
    /// switching between panes.
    Sidebar,
    /// Focus is on the main info pane, with information for the
    /// currently-selected binary visible
    ///
    /// Key events can affect either the tab bar or the info pane
    InfoPane
}

/// Application state
pub struct App<'a> {
    pub title : String,
    pub should_quit : bool,
    pub elf : &'a summarize::ElfSummary,
    pub resolved_dependencies : &'a collections::BTreeMap<String, Option<summarize::ElfSummary>>,
    pub selected_binary : ListState,
    pub focused_pane : Focus,
    /// The state of the tab widget for each binary
    ///
    /// This is initialized on demand
    pub binary_ui_state : collections::BTreeMap<PathBuf, BinaryUIState>
}

impl<'a> App<'a> {
    pub fn new(title : &str, elf_summary : &'a summarize::ElfSummary,
           resolved_deps : &'a collections::BTreeMap<String, Option<summarize::ElfSummary>>) -> Self {
        App {
            title : title.to_string(),
            should_quit : false,
            elf : elf_summary,
            resolved_dependencies : resolved_deps,
            selected_binary : ListState::default(),
            focused_pane : Focus::Sidebar,
            binary_ui_state : collections::BTreeMap::new()
        }
    }

    /// Get the tab state for the given binary; note that this is total because
    /// it will lazily instantiate tab state if needed.
    ///
    /// It takes the binary state is being requested for as evidence that a
    /// binary is selected; it does not rely on the binary selection in
    /// `selected_binary`.
    pub fn binary_ui_state(&mut self, bin : &'a summarize::ElfSummary) -> &mut BinaryUIState {
        self.binary_ui_state.entry(bin.filename.clone()).or_insert_with(BinaryUIState::new)
    }

    pub fn selected_binary(&self) -> Option<&'a summarize::ElfSummary> {
        match self.selected_binary.selected() {
            None => None,
            Some(idx) => {
                if idx == 0 {
                    Some(self.elf)
                } else {
                    let v = self.resolved_dependencies.values().map(|o| o.as_ref()).collect::<Vec<Option<&summarize::ElfSummary>>>();
                    v[idx - 1]
                }
            }
        }
    }

    pub fn on_key(&mut self, evt : KeyEvent) {
        match evt.code {
            KeyCode::Char('q') if evt.modifiers == KeyModifiers::CONTROL => {
                self.should_quit = true;
            },
            KeyCode::Char(c) if evt.modifiers == KeyModifiers::ALT  && c >= '1' && c <= '9' && self.focused_pane == Focus::InfoPane => {
                // c.is_ascii_digit()
                // The user wants to switch info pane using ALT+#
                match self.selected_binary() {
                    None => {},
                    Some(bin) => {
                        // If we are here at all, the tab state has been instantiated
                        match self.binary_ui_state.get_mut(&bin.filename) {
                            None => {},
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
            },
            KeyCode::Tab => {
                // Change focus between the sidebar and info pane
                if self.focused_pane == Focus::Sidebar {
                    self.focused_pane = Focus::InfoPane;
                } else {
                    self.focused_pane = Focus::Sidebar;
                }
            },
            KeyCode::Up if self.focused_pane == Focus::Sidebar => {
                let num_bins = 1 + self.resolved_dependencies.len();
                match self.selected_binary.selected() {
                    None => {
                        self.selected_binary.select(Some(num_bins - 1));
                    },
                    Some(sel_idx) if sel_idx == 0 => {
                        // No-op
                    },
                    Some(sel_idx) => {
                        self.selected_binary.select(Some(sel_idx - 1));
                    }
                }
            },
            KeyCode::Up if self.focused_pane == Focus::InfoPane => {
                match self.selected_binary() {
                    None => {},
                    Some(elf_summ) => {
                        let ui_state = self.binary_ui_state(elf_summ);
                        let selected_tab = ui_state.tab_state.tab_labels[ui_state.tab_state.selected_tab];
                        match selected_tab {
                            InfoTabLabels::DefinedDynamicSymbols => {
                                match &elf_summ.binary_type {
                                    summarize::BinaryType::Static => {},
                                    summarize::BinaryType::Dynamic(dyn_data) => {
                                        let num_items = dyn_data.provided_dynamic_symbols.len();
                                        match ui_state.defined_dynamic_table_state.selected() {
                                            None => {
                                                ui_state.defined_dynamic_table_state.select(Some(num_items - 1));
                                            },
                                            Some(0) => {},
                                            Some(cur_idx) => {
                                                ui_state.defined_dynamic_table_state.select(Some(std::cmp::max(0, cur_idx - 1)));
                                            }
                                        }
                                    }
                                }
                            },
                            InfoTabLabels::DynamicDependencies => {},
                            InfoTabLabels::Overview => {}
                        }
                    }
                }
            },
            KeyCode::Down if self.focused_pane == Focus::Sidebar => {
                let num_bins = 1 + self.resolved_dependencies.len();
                match self.selected_binary.selected() {
                    None => {
                        self.selected_binary.select(Some(0));
                    },
                    Some(sel_idx) => {
                        self.selected_binary.select(Some(std::cmp::min(sel_idx + 1, num_bins - 1)));
                    }
                }
            },
            KeyCode::Down if self.focused_pane == Focus::InfoPane => {
                match self.selected_binary() {
                    None => {},
                    Some(elf_summ) => {
                        let ui_state = self.binary_ui_state(elf_summ);
                        let selected_tab = ui_state.tab_state.tab_labels[ui_state.tab_state.selected_tab];
                        match selected_tab {
                            InfoTabLabels::DefinedDynamicSymbols => {
                                match &elf_summ.binary_type {
                                    summarize::BinaryType::Static => {},
                                    summarize::BinaryType::Dynamic(dyn_data) => {
                                        let num_items = dyn_data.provided_dynamic_symbols.len();
                                        match ui_state.defined_dynamic_table_state.selected() {
                                            None => {
                                                ui_state.defined_dynamic_table_state.select(Some(0));
                                            },
                                            Some(cur_idx) => {
                                                ui_state.defined_dynamic_table_state.select(Some(std::cmp::min(num_items - 1, cur_idx + 1)));
                                            }
                                        }
                                    }
                                }
                            },
                            InfoTabLabels::DynamicDependencies => {},
                            InfoTabLabels::Overview => {}
                        }
                    }
                }
            },
            _ => {}
        }
    }
}
