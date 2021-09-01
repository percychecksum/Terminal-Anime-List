use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};

const DB_PATH: &str = "./data/db.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize, Clone)]
struct Anime {
    name: String,
    year: usize,
    episodes: usize,
    watched: usize,
    status: String,
    rating: f32,
    score: u8,
    started: Date<Utc>,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Anime,
    Manga,
}

#[derive(Copy, Clone, Debug)]
enum CategoryItem {
    All,
    Watching,
    Completed,
    On Hold,
    Dropped,
    Plan To Watch,
}

#[derive(Copy, Clone, Debug)]
enum OptionItem {
    Edit,
    Quit,
}


impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Anime => 0,
            MenuItem::Manga => 1,
        }
    }
}

impl From<CategoryItem> for usize {
    fn from(input: CategoryItem) -> usize {
        match input {
            CategoryItem::All => 0,
            CategoryItem::Watching => 1,
            CategoryItem::Completed  => 2,
            CategoryItem::On Hold  => 3,
            CategoryItem::Dropped  => 4,
            CategoryItem::Plan To Watch  => 5,
        }
    }
}

impl From<OptionItem> for usize {
    fn from(input: OptionItem) -> usize {
        match input {
            OptionItem::Edit => 0,
            OptionItem::Quit => 1,
        }
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Anime", "Manga"];
    let category_titles = vec!["All", "Watching", "Completed", "On Hold", "Dropped", "Plan to watch"];
    let option_titles = vec!["Edit", "Quit"];
    let mut active_menu_item = MenuItem::Anime;
    let mut active_category_item = MenuItem::All;

    let mut anime_list_state = ListState::default();
    anime_list_state.select(Some(0));
    let mut manga_list_state = ListState::default();
    manga_list_state.select(Some(0));

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                    ]
                    .as_ref(),
                )
                .split(size);
            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();
            let category = category_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();
            let option = option_titles
                .iter()
                .map(|t| {
                    let (first, reset) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),                    ])
                })
                .collect();

            let menu_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)].asf_ref(),
                )
                .split(chunks[0]);

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::LightCyan))
                .divider(Span::raw("|"));
            rect.render_widget(tabs, menu_chunks[0]);
            let categories = Tabs::new(category)
                .select(active_category_item.into())
                .block(Block::default().title("Status").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::LightCyan))
                .divider(Span::raw("|"));
            rect.render_widget(categories, menu_chunks[1]);
            let options = Tabs::new(menu)
                .block(Block::default().title("Options").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::LightCyan))
                .divider(Span::raw("|"));
            rect.render_widget(options, menu_chunks[0]);
            match active_menu_item {
                MenuItem::Anime => rect.render_stateful_widget(render_anime(), chunks[1], &mut anime_list_state),
                MenuItem::Manga => rect.render_stateful_widget(render_manga(), chunks[1], &mut manga_list_state),
            }
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('a') => active_menu_item = MenuItem::Anime,
                KeyCode::Char('m') => active_menu_item = MenuItem::Manga,
                KeyCode::Char('e') => {
                    if (active_category_item == Anime) {
                        edit_anime_at_index(&mut anime_list_state).expect("can edit anime entry");
                    } else {
                        edit_manga_at_indec(&mut manga_list_state).expect("can edit manga entry");
                    }
                }
                KeyCode::Char('q') => {
                    if (active_category_item == Anime) {
                        remove_anime_at_index(&mut anime_list_state).expect("can remove anime entry");
                    } else {
                        remove_manga_at_indec(&mut manga_list_state).expect("can remove manga entry");
                    }
                }
                KeyCode::Char('+') => {
                    if (active_category_item == Anime) {
                        add_episode_at_index(&mut anime_list_state).expect("can add episode of anime");
                    } else {
                        add_book_at_index(&mut manga_list_state).expect("can add book of manga");
                    }
                }
                KeyCode::Char('-') => {
                    if (active_category_item == Anime) {
                        remove_episode_at_index(&mut anime_list_state).expect("can remove episode of anime");
                    } else {
                        remove_book_at_index(&mut manga_list_state).expect("can remove book of manga");
                    }                }
                KeyCode::Left => active_category_item = CategoryItem::-1, // !!!
                KeyCode::Right => active_category_item = CategoryItem::+1, // !!!
                KeyCode::Down => {
                    if (active_category_item == Anime) {
                        if let Some(selected) = anime_list_state.selected() {
                            let amount_anime = read_db().expect("can fetch anime list").len();
                            if selected >= amount_anime - 1 {
                                anime_list_state.select(Some(0));
                            } else {
                                anime_list_state.select(Some(selected + 1));
                            }
                        }
                    } else {
                        if let Some(selected) = manga_list_state.selected() {
                            let amount_manga = read_db().expect("can fetch manga list").len();
                            if selected >= amount_manga - 1 {
                                manga_list_state.select(Some(0));
                            } else {
                                manga_list_state.select(Some(selected + 1));
                            }
                        }

                    }

                }
                KeyCode::Up => {
                    if (active_category_item == Anime) {
                        if let Some(selected) = anime_list_state.selected() {
                            let amount_anime = read_db().expect("can fetch anime list").len();
                            if selected > 0 {
                                anime_list_state.select(Some(selected - 1));
                            } else {
                                anime_list_state.select(Some(amount_anime - 1));
                            }
                        }
                    } else {
                        if let Some(selected) = manga_list_state.selected() {
                            let amount_manga = read_db().expect("can fetch manga list").len();
                            if selected > 0 {
                                manga_list_state.select(Some(selected - 1));
                            } else {
                                manga_list_state.select(Some(amount_manga - 1));
                            }
                        }

                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }
}