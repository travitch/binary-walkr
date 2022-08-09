use crate::ui::app::{App, InfoTabLabels};
use crate::summarize::{BinaryType, ElfSummary};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans,Text},
    widgets::{
        Block, Borders, List, ListItem,
        Paragraph, Row, Table, Tabs,
    },
    Frame,
};

fn draw_binary_list_sidebar<B: Backend>(f : &mut Frame<B>, app: &mut App, area : Rect) {
    let mut items = vec![ListItem::new(Text::from(app.elf.filename.as_path().to_string_lossy()))];

    for lib in app.resolved_dependencies.keys() {
        items.push(ListItem::new(Text::from(format!("  {}", lib))));
    }

    let w = List::new(items)
        .block(Block::default().title("Binary Images").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");
    f.render_stateful_widget(w, area, &mut app.selected_binary);
}

fn draw_endian(end : object::Endianness) -> &'static str {
    match end {
        object::Endianness::Little => "little",
        object::Endianness::Big => "big"
    }
}

fn draw_binary_overview<B: Backend>(f : &mut Frame<B>, elf_summ : &ElfSummary, area : Rect) {
    let overview_data = vec![
        Row::new(vec![String::from("Path:"), elf_summ.filename.as_path().to_string_lossy().into_owned()]),
        Row::new(vec!["Endianness:", draw_endian(elf_summ.endianness)]),
        Row::new(vec![String::from("Pointer Width: "), format!("{} bits", elf_summ.bit_size)])
    ];
    let overview = Table::new(overview_data)
        .column_spacing(1)
        .widths(&[Constraint::Min(15), Constraint::Ratio(5, 6)])
        .block(Block::default().title("Overview").borders(Borders::ALL));
    f.render_widget(overview, area);
}

fn draw_defined_dynamic_symbols<B: Backend>(f : &mut Frame<B>, elf_summ : &ElfSummary, area : Rect) {
    match &elf_summ.binary_type {
        BinaryType::Static => {
            let w = Paragraph::new("No dynamic symbols (static binary)");
            f.render_widget(w, area);
        },
        BinaryType::Dynamic(dyn_data) if dyn_data.provided_dynamic_symbols.is_empty() => {
            let w = Paragraph::new("No dynamic symbols defined");
            f.render_widget(w, area);
        },
        BinaryType::Dynamic(dyn_data) => {
            let mut defined_sym_data = Vec::new();

            for sym_def in &dyn_data.provided_dynamic_symbols {
                defined_sym_data.push(Row::new(vec![
                    format!("{:#x}", sym_def.address),
                    format!("{}", sym_def.size),
                    format!("{:?}", sym_def.type_),
                    format!("{:?}", sym_def.binding),
                    String::from(&sym_def.symbol.name)
                ]));
            }

            let defined_sym_view = Table::new(defined_sym_data)
                .column_spacing(1)
                .widths(&[Constraint::Min(11), Constraint::Min(5), Constraint::Min(12), Constraint::Min(12), Constraint::Length(40)])
                .block(Block::default().title("Defined Dynamic Symbols").borders(Borders::ALL))
                .header(
                    Row::new(vec!["Address", "Size", "Type", "Binding", "Symbol"])
                        .style(Style::default().fg(Color::Yellow))
                        .bottom_margin(1)
                );
            f.render_widget(defined_sym_view, area);
        }
    }
}

fn draw_selected_binary<B: Backend>(f : &mut Frame<B>, app : &mut App, area : Rect) {
    match app.selected_binary() {
        None => {},
        Some(elf_summ) => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(40)].as_ref())
                .split(area);

            let tab_state = app.binary_tab_state(elf_summ);
            let titles = tab_state.tab_labels
                .iter()
                .map(|l| Spans::from(l.to_string()))
                .collect();
            let tabs = Tabs::new(titles)
                .block(Block::default().title("Binary Views").borders(Borders::ALL))
                .highlight_style(Style::default().fg(Color::Yellow))
                .select(tab_state.selected_tab)
                .divider(Span::from("|"));

            f.render_widget(tabs, chunks[0]);

            match tab_state.tab_labels[tab_state.selected_tab] {
                InfoTabLabels::Overview => {
                    draw_binary_overview(f, elf_summ, chunks[1]);
                },
                InfoTabLabels::DynamicDependencies => {},
                InfoTabLabels::DefinedDynamicSymbols => {
                    draw_defined_dynamic_symbols(f, elf_summ, chunks[1]);
                }
            }
        }
    }
}

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Set up a two column layout; the left will be the list of binary images,
    // while the right will be details for the selected image
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)].as_ref())
        .split(f.size());

    draw_binary_list_sidebar(f, app, chunks[0]);
    draw_selected_binary(f, app, chunks[1]);
}
