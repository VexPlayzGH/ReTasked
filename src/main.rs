use color_eyre::eyre::{Ok, Result};
use ratatui::{
    DefaultTerminal, Frame, crossterm::event::{self, Event, KeyEvent},
    layout::{Alignment, Constraint, Layout}, prelude::Direction,
    style::{Color, Modifier, Style, Stylize},
    text::ToSpan, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Widget}};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct AppState { // Store each App state into a struct
    items: Vec<TodoItem>,
    list_state: ListState,
    is_add_new: bool,
    is_help: bool,
    input_value: String,
    show_save_message: bool,
    save_message_time: Option<Instant>,
}

#[derive(Debug, Default)]
struct TodoItem {
    is_done: bool,
    description: String,
}

enum FormAction {
    None,
    Submit,
}

fn main() -> Result<()> {
    let mut state = AppState::default();
    state.is_add_new = false;
    color_eyre::install()?;

    let terminal = ratatui::init();
    let _ = get_prev_list(&mut state);
    let result = run(terminal, &mut state);
    ratatui::restore();

    result
}

fn run(mut terminal: DefaultTerminal, app_state: &mut AppState) -> Result<()> { // runs every frame and calls event handlers
    loop {
        //Rendering
        terminal.draw(|f| render(f, app_state))?;
        //Input Handling
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app_state.is_add_new {
                    match handle_add_new(key, app_state) {
                        FormAction::None => {},
                        FormAction::Submit => {
                            app_state.is_add_new = false;
                            app_state.items.push(TodoItem {
                                is_done: false,
                                description: format!("• {}", app_state.input_value.clone())
                            });
                            app_state.input_value.clear();
                        },
                    }
                }
                else if app_state.is_help {
                    handle_help(key, app_state);
                }
                else {
                    if handle_key(key, app_state) {
                        break;
                    }
                }            
            }
        }
    }

    Ok(())
}

fn write_file(file_path: &str, contents: &str) -> Result<()> { // writes to a file (used for the list save)
    let mut file = File::create(file_path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

fn handle_help(key: KeyEvent, app_state: &mut AppState) { // handles the help menu toggle
    match key.code {
        event::KeyCode::Esc => {
            if key.kind == event::KeyEventKind::Press {
                app_state.is_help = false;                
            }            
        },
        _ => {},
    }    
}

fn handle_add_new(key: KeyEvent, app_state: &mut AppState) -> FormAction { // handles K/B input on Input Mode
    match key.code {
        event::KeyCode::Char(c) => {
            if key.kind == event::KeyEventKind::Press {
                app_state.input_value.push(c);
            }            
        },
        event::KeyCode::Backspace => {
            if key.kind == event::KeyEventKind::Press {
                app_state.input_value.pop();
            }            
        },
        event::KeyCode::Enter => {
            if key.kind == event::KeyEventKind::Press {
                return FormAction::Submit;
            }            
        },
        event::KeyCode::Esc => {
            if key.kind == event::KeyEventKind::Press {
                app_state.is_add_new = false;
                app_state.input_value.clear();
                return FormAction::None;
            }            
        },
        _ => {},
    }
    FormAction::None
}
fn handle_key(key: KeyEvent, app_state: &mut AppState) -> bool { // Handles K/B input on View Mode
    match key.code { 
        event::KeyCode::F(1) | event::KeyCode::Char('H') | event::KeyCode::Char('h') => {
            if key.kind == event::KeyEventKind::Press {
                app_state.is_help = true;
            }
        },
        event::KeyCode::Enter => {
            if key.kind == event::KeyEventKind::Press {
                if let Some(index) = app_state.list_state.selected() {
                    if let Some(item) = app_state.items.get_mut(index) {
                        item.is_done = !item.is_done;
                    }
                }
            }
        },
        event::KeyCode::Esc => {
            if key.kind == event::KeyEventKind::Press {
                return true;
            }            
        },
        event::KeyCode::Char(char) => match char {
            'o' | 'O' => {
                if key.kind == event::KeyEventKind::Press {
                    app_state.items.clear();
                    match write_file("output.txt", "") {
                        _ => {},
                    }
                }
            },
            's' | 'S' => {
                if key.kind == event::KeyEventKind::Press {
                    let contents = app_state.items
                        .iter()
                        .map(|item| format!("{}\n", item.description))
                        .collect::<String>();
                    match write_file("output.txt", &contents) {
                        _ => {},
                    }
                    app_state.show_save_message = true;
                    app_state.save_message_time = Some(Instant::now());
                }  
            },
            'a' | 'A' => {
                if key.kind == event::KeyEventKind::Press {
                    app_state.is_add_new = true;
                }                
            },
            'd' | 'D' => {
                if key.kind == event::KeyEventKind::Press {
                    if let Some(index) = app_state.list_state.selected() {
                        app_state.items.remove(index);
                    }
                }                
            },
            'k' | 'K' => {
                if key.kind == event::KeyEventKind::Press {
                    app_state.list_state.select_next();
                }
            },
            'j' | 'J' => {
                if key.kind == event::KeyEventKind::Press {
                    app_state.list_state.select_previous();
                }
            },
            _ => {},
        },
        event::KeyCode::Up => {
            if key.kind == event::KeyEventKind::Press {
                app_state.list_state.select_previous();
            }
        },
        event::KeyCode::Down => {
            if key.kind == event::KeyEventKind::Press {
                app_state.list_state.select_next();
            }
        },
        _ => {},
    }
    false
}

fn get_prev_list(app_state: &mut AppState) -> Result<()> { // Manages the display of the already stored list in "output.txt"
    let file_path = Path::new("output.txt");
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    for line_result in reader.lines() {  // Remove leading and trailing whitespace
        let line = line_result?;
        app_state.items.push(TodoItem {
                            is_done: false,
                            description: line,
                        });
    }
    Ok(())
}

fn render(frame: &mut Frame, app_state: &mut AppState) { // renders all widgets  
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(33)
        ])
        .split(frame.area());
    
    let [border_area] = Layout::vertical([Constraint::Fill(1)])
            .margin(2)
            .areas(frame.area());
    if app_state.is_add_new {
        Paragraph::new(app_state.input_value.as_str())
            .block(Block::bordered()
                .title("INPUT MODE // Add new task".to_span().into_centered_line())
                .fg(Color::Green)
                .padding(Padding::uniform(2))
                .border_type(BorderType::Rounded))
            .render(border_area, frame.buffer_mut());
    }
    else if app_state.is_help {
        Paragraph::new(app_state.input_value.as_str())
            .block(Block::bordered()
                .fg(Color::Green)
                .padding(Padding::uniform(2))
                .border_type(BorderType::Rounded))
            .render(border_area, frame.buffer_mut());
        frame.render_widget(
            Paragraph::new("  [VIEW MODE]\n\n  A - Add\n  D - Delete\n  J / UpArrow - Select previous\n  \
            K / DownArrow - Select next\n  S - Save list\n  O - Clear list\n  Enter - Mark as complete\n  Esc - Exit program\n\n")
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .fg(Color::Rgb(0, 255, 255))
                        .padding(Padding::uniform(2))
                        .border_type(BorderType::Rounded)
                ),
            layout[0]
        );
        frame.render_widget(
            Paragraph::new("[INPUT MODE]\n\nEnter - Insert\nEsc - Cancel\n\n")
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .title("KEYBINDS".to_span().into_centered_line())
                        .fg(Color::Rgb(0, 255, 255))
                        .padding(Padding::uniform(2))
                        .border_type(BorderType::Rounded)
                ),
            layout[1]
        );
        frame.render_widget(
            Paragraph::new("[KEYBINDS MENU]\n\nEsc - Return back")
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .fg(Color::Rgb(0, 255, 255))
                        .padding(Padding::uniform(2))
                        .border_type(BorderType::Rounded)
                ),
            layout[2]
        );
    }
    else {        
        let [inner_area] = Layout::vertical([Constraint::Fill(1)])
            .margin(2)
            .areas(border_area);

        Block::bordered().border_type(BorderType::Rounded)
            .title("VIEW MODE // View the list // Press F1/H to show keybinds".to_span().into_centered_line())
            .fg(Color::Rgb(255, 255, 0))
            .render(border_area, frame.buffer_mut());
        
        let list_area = if app_state.show_save_message {
            if let Some(show_time) = app_state.save_message_time {
                if show_time.elapsed().as_secs() >= 3 {
                    app_state.show_save_message = false;
                    app_state.save_message_time = None;
                }
            }
            Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1)
                ])
                .split(inner_area)[0]
        } else {
            inner_area
        };

        let list = List::new(app_state.items
            .iter()
            .map(|x| {
                let value = if x.is_done {
                   x.description.to_span().add_modifier(Modifier::BOLD)
                   .style(Style::default().fg(Color::Rgb(255, 0, 0))).crossed_out()
                } else {
                   x.description.to_span() 
                };
                ListItem::from(value)
            }))
                .highlight_symbol(">")
                .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC).fg(Color::Green));

        frame.render_stateful_widget(list, list_area, &mut app_state.list_state);

        if app_state.show_save_message {
            if let Some(show_time) = app_state.save_message_time {
                if show_time.elapsed().as_secs() < 3 { // keeps the notification on save displayed for 3 seconds
                    let message_area = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
                        .split(inner_area)[1];
                    frame.render_widget(
                        Paragraph::new("List Saved!")
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(Color::Rgb(255, 255, 0))),
                        message_area
                    );
                }
            }
        }
    }
}