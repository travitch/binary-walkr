use crate::ui::app::App;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans,Text},
    widgets::{
        Block, Borders, List, ListItem,
        Row, Table, Tabs,
    },
    Frame,
};

fn draw_binary_list_sidebar<B: Backend>(f : &mut Frame<B>, app: &mut App, area : Rect) {
    let mut items = Vec::new();

    items.push(ListItem::new(Text::from(app.elf.filename.as_path().to_string_lossy())));

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

fn draw_selected_binary<B: Backend>(f : &mut Frame<B>, app : &mut App, area : Rect) {
    match app.selected_binary() {
        None => {},
        Some(elf_summ) => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(40)].as_ref())
                .split(area);
            let titles = ["Overview", "Dynamic Dependencies", "Defined Dynamic Symbols"].iter().cloned().map(Spans::from).collect();
            let tabs = Tabs::new(titles)
                .block(Block::default().title("Binary Views").borders(Borders::ALL))
                .highlight_style(Style::default().fg(Color::Yellow))
                .select(0)
                .divider(Span::from("|"));

            f.render_widget(tabs, chunks[0]);

            let overview_data = vec![
                Row::new(vec!["Endianness: ", draw_endian(elf_summ.endianness)]),
                Row::new(vec![format!("{}-bit Elf", elf_summ.bit_size), String::from("")])
            ];
            let overview = Table::new(overview_data)
                .column_spacing(1)
                .widths(&[Constraint::Min(10), Constraint::Min(10)])
                .block(Block::default().title("Overview").borders(Borders::ALL));
            f.render_widget(overview, chunks[1]);
        }
    }
}

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Set up a two column layout; the left will be the list of binary images,
    // while the right will be details for the selected image
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)].as_ref())
        // .constraints([Constraint::Length(40), Constraint::Min(0)].as_ref())
        .split(f.size());

    draw_binary_list_sidebar(f, app, chunks[0]);
    draw_selected_binary(f, app, chunks[1]);
}
