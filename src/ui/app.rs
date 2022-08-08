use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections;
use tui::widgets::ListState;

use crate::summarize;

/// The focused component of the UI (i.e., the component receiving keystrokes that are not global)
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub enum Focus {
    /// Focus is on the sidebar; note that the selected item is stored in the
    /// `selected_binary` `ListState`, as we don't want to lose that state when
    /// switching between panes.
    Sidebar
}

/// Application state
pub struct App<'a> {
    pub title : String,
    pub enhanced_graphics : bool,
    pub should_quit : bool,
    pub elf : &'a summarize::ElfSummary,
    pub resolved_dependencies : &'a collections::BTreeMap<String, Option<summarize::ElfSummary>>,
    pub selected_binary : ListState,
    pub focused_pane : Focus
}

impl<'a> App<'a> {
    pub fn new(title : &str, enhanced_graphics : bool, elf : &'a summarize::ElfSummary,
           resolved_deps : &'a collections::BTreeMap<String, Option<summarize::ElfSummary>>) -> Self {
        App {
            title : title.to_string(),
            enhanced_graphics : enhanced_graphics,
            should_quit : false,
            elf : elf,
            resolved_dependencies : resolved_deps,
            selected_binary : ListState::default(),
            focused_pane : Focus::Sidebar
        }
    }

    pub fn selected_binary(&self) -> Option<&'a summarize::ElfSummary> {
        match self.selected_binary.selected() {
            None => None,
            Some(idx) => {
                if idx == 0 {
                    Some(&self.elf)
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
            }
            _ => {}
        }
    }
}
