use ansi_to_tui::IntoText;
use anyhow::Result;
use crossterm::{
  event::{DisableMouseCapture, EnableMouseCapture},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io;
use std::io::{Read, Stdout};
use tui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Span, Spans},
  widgets::{Block, Borders, List, ListItem},
  Frame, Terminal,
};

use crate::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
  Messages,
  Errors,
  Unprefixed,
  // Help,
}

impl DisplayState {
  pub fn next(&self) -> DisplayState {
    match self {
      DisplayState::Messages => DisplayState::Errors,
      DisplayState::Errors => DisplayState::Unprefixed,
      DisplayState::Unprefixed => DisplayState::Messages,
    }
  }
}

pub fn draw(app: &mut App, f: &mut Frame<CrosstermBackend<io::Stdout>>) {
  let size = f.size();
  let main_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Min(0), Constraint::Length(30)].as_ref())
    .split(size);

  let right_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(4)].as_ref())
    .split(main_chunks[1]);

  render_messages(app, f, main_chunks[0]);
  render_prefix_list(app, f, right_chunks[0]);
  render_other_list(app, f, right_chunks[1]);

  ()
}

fn render_messages(app: &mut App, f: &mut Frame<CrosstermBackend<io::Stdout>>, destination: Rect) {
  let height = f.size().height.into();
  let messages = app
    .get_current_messages(height)
    .iter()
    .map(|s| ListItem::new(s.clone().into_bytes().into_text().unwrap()))
    .collect::<Vec<ListItem>>();

  match app.display_state {
    DisplayState::Messages => {
      let prefix = app.get_selected_prefix();
      let title = if let Some(p) = prefix {
        format!(" Messages for {} ", p)
      } else {
        " Messages ".to_string()
      };
      let list = List::new(messages)
        .block(
          Block::default()
            .borders(Borders::ALL)
            .title(Spans::from(vec![Span::styled(
              title,
              Style::default().fg(Color::Green),
            )])),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
      f.render_widget(list, destination);
    }
    DisplayState::Errors => {
      let errors: Vec<ListItem> = app
        .error_messages
        .get_messages(height)
        .iter()
        .map(|s| ListItem::new(s.to_string()))
        .collect::<Vec<ListItem>>();

      let error_list = List::new(errors).block(
        Block::default()
          .borders(Borders::ALL)
          .title("Errors")
          .style(Style::default().fg(Color::Red)),
      );
      f.render_widget(error_list, destination);
    }
    DisplayState::Unprefixed => {
      let list = List::new(
        app
          .unprefixed_messages
          .get_messages(height)
          .iter()
          .map(|s| ListItem::new(s.message.clone().into_bytes().into_text().unwrap()))
          .collect::<Vec<ListItem>>(),
      )
      .block(Block::default().borders(Borders::ALL).title(" no parse "));
      f.render_widget(list, destination);
    }
  }
}

fn render_prefix_list(
  app: &mut App,
  f: &mut Frame<CrosstermBackend<io::Stdout>>,
  destination: Rect,
) {
  let titles = app.get_buckets();
  let titles: Vec<Spans> = titles
    .iter()
    .map(|(count, label)| {
      Spans(vec![
        Span::styled(
          format!("{:3} ", count),
          Style::default().fg(if count > &0 {
            Color::Cyan
          } else {
            Color::White
          }),
        ),
        Span::styled(label, Style::default().fg(Color::White)),
      ])
    })
    .collect();

  let tabs = List::new(
    titles
      .iter()
      .map(|s| ListItem::new(s.clone()))
      .collect::<Vec<ListItem>>(),
  )
  .highlight_style(
    Style::default()
      .add_modifier(Modifier::BOLD)
      .bg(Color::Blue),
  )
  .block(
    Block::default()
      .borders(Borders::ALL)
      .title("Prefixes")
      .style(Style::default().fg(Color::White)),
  );

  f.render_stateful_widget(tabs, destination, &mut app.list_state.clone());
}

fn render_other_list(app: &mut App, f: &mut Frame<CrosstermBackend<io::Stdout>>, target: Rect) {
  // debug!("target: {:?}", target);
  let error_style = if app.error_messages.new_messages > 0 {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
  } else if app.display_state == DisplayState::Errors {
    Style::default().fg(Color::Red)
  } else {
    Style::default().fg(Color::White)
  };
  let unprefixed_style = if app.unprefixed_messages.new_messages > 0 {
    Style::default()
      .fg(Color::Yellow)
      .add_modifier(Modifier::BOLD)
  } else if app.display_state == DisplayState::Unprefixed {
    Style::default().fg(Color::Yellow)
  } else {
    Style::default().fg(Color::White)
  };
  let list = List::new(vec![
    ListItem::new(Spans(vec![
      Span::styled(
        format!("{:3} ", app.error_messages.new_messages),
        Style::default().fg(Color::Yellow),
      ),
      Span::styled("stderr", error_style),
    ])),
    ListItem::new(Spans(vec![
      Span::styled(
        format!("{:3} ", app.unprefixed_messages.new_messages),
        Style::default().fg(Color::Yellow),
      ),
      Span::styled("no parse", unprefixed_style),
    ])),
  ])
  .block(
    Block::default()
      .borders(Borders::ALL)
      .title("Other")
      .style(error_style),
  );

  f.render_widget(list, target);
}

pub fn setup_tui() -> Result<(), std::io::Error> {
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
}

pub fn teardown_tui(
  terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), std::io::Error> {
  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    DisableMouseCapture
  )?;
  terminal.show_cursor()
}