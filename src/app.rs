use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use crossterm::terminal::ScrollUp;
use log::debug;
use log::info;
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io::{LineWriter, Write};
use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::{collections::HashMap, time::Duration};
use std::{collections::LinkedList, io::Stdout};
use std::{env, fs};
use tempfile::NamedTempFile;
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use crate::cli::Config;
use crate::render::DisplayState;
use crate::{bucket::Bucket, render::draw};

#[derive(Clone, Debug)]
pub enum AppMessage {
  Exit(ExitStatus),
}

impl Line {
  pub fn with_prefix(prefix: String, message: String, has_error: bool) -> Self {
    Self {
      prefix: Some(prefix),
      message,
      has_error,
    }
  }
  pub fn without_prefix(message: String) -> Self {
    Self {
      prefix: None,
      message,
      ..Default::default()
    }
  }
}

pub struct App {
  /// Messages by prefix
  pub buckets: HashMap<String, Bucket>,
  pub error_messages: Bucket,
  pub unprefixed_messages: Bucket,
  pub list_state: ListState,
  pub display_state: DisplayState,
  config: Config,
  regex: Regex,
  error_regex: Regex,
  pub exit_code: Option<ExitStatus>,
}

#[derive(Clone, Debug, Default)]
pub struct Line {
  pub prefix: Option<String>,
  pub message: String,
  pub has_error: bool,
}

impl Line {
  pub fn render(&self) -> String {
    return format!(
      "{}{}",
      self
        .prefix
        .as_ref()
        .map(|p| format!("{}: ", p))
        .unwrap_or_default(),
      self.message
    );
  }
}

impl App {
  pub fn new(config: &Config) -> App {
    App {
      display_state: DisplayState::Messages,
      buckets: HashMap::new(),
      error_messages: Bucket::new(),
      unprefixed_messages: Bucket::new(),
      list_state: {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        list_state
      },
      config: config.clone(),
      regex: Regex::new(config.prefix.as_str()).unwrap(),
      error_regex: Regex::new(r"(?i).*(error|exception|stack.?trace).*").unwrap(),
      exit_code: None,
    }
  }

  pub fn run(
    &mut self,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    output: Receiver<String>,
    errors: Receiver<String>,
    monitor: Receiver<AppMessage>,
  ) -> Result<(), std::io::Error> {
    loop {
      let height = terminal.size()?.height;
      let now = Instant::now();
      let stdout_end = now + Duration::from_millis(4);
      let stderr_end = now + Duration::from_millis(8);
      let render_end = now + Duration::from_millis(16);
      output
        .try_iter()
        .take_while(|_| Instant::now() < stdout_end)
        .for_each(|l| {
          self
            .parse_line(&l)
            .into_iter()
            .for_each(|parsed| self.process_line(&parsed));
        });
      errors
        .try_iter()
        .take_while(|_| Instant::now() < stderr_end)
        .for_each(|l| self.process_error(&l));

      let remaining = render_end - Instant::now();
      terminal.draw(|f| draw(self, f))?;
      if let Ok(x) = monitor.try_recv() {
        info!("Process exited: {:?}", x);
        match x {
          AppMessage::Exit(code) => {
            self.notify_exit(code);
          }
        }
      }
      if event::poll(remaining)? {
        let event = event::read()?;
        match event {
          Event::Key(key) => match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char('c') if key.modifiers & KeyModifiers::CONTROL > KeyModifiers::NONE => {
              return Ok(())
            }
            KeyCode::Char('j') => self.next_prefix(),
            KeyCode::Char('k') => self.previous_prefix(),
            KeyCode::Char('w') => self.scroll_up(height.into()),
            KeyCode::Char('s') => self.scroll_down(height.into()),
            KeyCode::Char('K') => self.scroll_up(height.into()),
            KeyCode::Char('J') => self.scroll_down(height.into()),
            KeyCode::Char('r') => self.scroll_reset(),
            KeyCode::Esc => self.set_display_state(DisplayState::Messages),
            KeyCode::Char('e') => self.set_display_state(DisplayState::Errors),
            KeyCode::Char('p') => self.set_display_state(DisplayState::ParseErrors),
            KeyCode::Char('n') => self.next_bucket(),
            KeyCode::Char('c') => self.clear_current_bucket(),
            KeyCode::Char('C') => self.clear_all_buckets(),
            KeyCode::Enter => self.open_in_editor().unwrap_or(()),
            _ => {}
          },
          Event::Mouse(mouse) => match mouse {
            MouseEvent {
              kind: MouseEventKind::ScrollUp,
              ..
            } => self.scroll_up(height.into()),
            MouseEvent {
              kind: MouseEventKind::ScrollDown,
              ..
            } => self.scroll_down(height.into()),
            _ => {}
          },
          _ => (),
        }
      }
    }
  }

  fn notify_exit(&mut self, exit_code: ExitStatus) {
    self.exit_code = Some(exit_code);
  }

  fn scroll_up(&mut self, height: usize) {
    if let Some(bucket) = self.get_current_bucket() {
      bucket.scroll_up(height);
    }
  }

  fn scroll_down(&mut self, height: usize) {
    if let Some(bucket) = self.get_current_bucket() {
      bucket.scroll_down(height);
    }
  }

  fn scroll_reset(&mut self) {
    if let Some(bucket) = self.get_current_bucket() {
      bucket.scroll_reset();
    }
  }

  fn set_display_state(&mut self, state: DisplayState) {
    if self.display_state != state {
      self.display_state = state;
    } else {
      self.display_state = DisplayState::Messages;
    }
  }

  fn next_prefix(&mut self) {
    if self.buckets.len() == 0 {
      return;
    }
    self.list_state.select(
      self
        .list_state
        .selected()
        .map(|i| (i + 1) % self.buckets.len()),
    );
  }
  fn previous_prefix(&mut self) {
    if self.buckets.len() == 0 {
      return;
    }
    self.list_state.select(
      self
        .list_state
        .selected()
        .map(|i| (i + self.buckets.len() - 1) % self.buckets.len()),
    );
  }

  fn process_line(&mut self, line: &Line) {
    if let Some(prefix) = &line.prefix {
      if let Some(bucket) = self.buckets.get_mut(prefix) {
        bucket.add_message(line.clone())
      } else {
        self.buckets.insert(
          prefix.to_string(),
          Bucket::from_messages(vec![line.clone()]),
        );
      }
    } else {
      self.unprefixed_messages.add_message(line.clone());
    }
  }

  fn parse_line(&self, line: &String) -> Option<Line> {
    debug!("Parsing line: {}", line);
    let input = line.trim();
    let res: Option<Line>;
    if let Some(caps) = self.regex.captures(line) {
      if caps.len() >= 2 {
        let has_error = self.error_regex.is_match(line);
        res = Some(Line::with_prefix(
          caps[1].to_string(),
          caps[2].to_string(),
          has_error,
        ));
      } else {
        debug!("No prefix found for line: {}", line);
        res = Some(Line::without_prefix(input.to_string()))
      }
      debug!("Parsed line: {:?}", res);
    } else {
      res = Some(Line::without_prefix(input.to_string()))
    }
    res
  }

  fn process_error(&mut self, error: &String) {
    self
      .error_messages
      .add_message(Line::without_prefix(error.to_string()));
  }

  pub fn get_buckets(&self) -> Vec<(&String, &Bucket)> {
    let mut vec = self.buckets.iter().collect::<Vec<_>>();
    vec.sort_by_key(|(s, _)| s.clone());
    vec
  }

  pub fn get_selected_prefix(&self) -> Option<String> {
    self
      .list_state
      .selected()
      .and_then(|i| {
        if i >= self.buckets.len() {
          None
        } else {
          Some(i)
        }
      })
      .map(|i| self.get_buckets()[i].0.clone())
  }

  pub fn get_current_bucket(&mut self) -> Option<&mut Bucket> {
    self
      .get_selected_prefix()
      .and_then(|prefix| self.buckets.get_mut(&prefix))
  }

  pub fn get_current_messages(&mut self, count: usize) -> LinkedList<String> {
    if self.buckets.len() == 0 {
      return LinkedList::new();
    }
    let mut bucket = self.get_current_bucket().unwrap();
    bucket
      .get_messages(count - 2)
      .iter()
      .map(|l| l.message.clone())
      .collect()
  }

  fn open_in_editor(&mut self) -> Option<()> {
    let mut prefix_name = self.get_selected_prefix()?;
    let fixed_prefix = Regex::new(r"[@\-/\\:]")
      .unwrap()
      .replace_all(&prefix_name, "_");

    let log_lines: Vec<String> = self
      .get_current_bucket()?
      .get_all_messages()
      .iter()
      .map(|l| l.render())
      .collect();
    let log = log_lines.join("\n");
    let filename = format!("/tmp/{}.log", fixed_prefix);
    let mut file = OpenOptions::new()
      .write(true)
      .create(true)
      .open(filename.as_str())
      .ok()?;
    file.write_all(log.as_bytes()).ok()?;
    info!("Wrote log to file {}", filename);

    let editor = env::var("EDITOR").ok()?;
    let mut args = vec![];
    args.push("-n");
    args.push(&filename);

    let mut command = Command::new(editor);
    command.stdout(Stdio::null());
    command.args(args);
    command.spawn().ok()?;
    //
    return Some(());
  }

  fn next_bucket(&mut self) {
    let buckets = self.get_buckets();
    let selected = self.list_state.selected().unwrap_or(0);
    let end = selected + buckets.len();
    for n in selected + 1..end {
      let i = n % buckets.len();
      let bucket = &buckets[i];
      if bucket.1.new_errors > 0 {
        self.list_state.select(Some(i));
        return;
      }
    }
    for n in selected + 1..end {
      let i = n % buckets.len();
      let bucket = &buckets[i];
      if bucket.1.new_messages > 0 {
        self.list_state.select(Some(i));
        return;
      }
    }
  }

  fn clear_all_buckets(&mut self) {
    for (_, bucket) in self.buckets.iter_mut() {
      bucket.clear_all_messages();
    }
  }

  fn clear_current_bucket(&mut self) {
    if let Some(bucket) = self.get_current_bucket() {
      bucket.clear_all_messages();
    }
  }
}
