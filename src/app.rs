use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use log::{debug};
use regex::Regex;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::{collections::HashMap, time::Duration};
use std::{collections::LinkedList, io::Stdout};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use crate::cli::Config;
use crate::render::DisplayState;
use crate::{bucket::Bucket, render::draw};

impl Line {
  pub fn with_prefix(prefix: String, message: String) -> Self {
    Self {
      prefix: Some(prefix),
      message,
    }
  }
  pub fn without_prefix(message: String) -> Self {
    Self {
      prefix: None,
      message,
    }
  }
}

pub struct App {
  /// Messages by prefix
  pub buckets: HashMap<String, Bucket<Line>>,
  pub error_messages: Bucket<String>,
  pub unprefixed_messages: Bucket<Line>,
  pub list_state: ListState,
  pub display_state: DisplayState,
  config: Config,
  regex: Regex,
}

#[derive(Clone, Debug, Default)]
pub struct Line {
  pub prefix: Option<String>,
  pub message: String,
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
    }
  }

  pub fn run(
    &mut self,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    output: Receiver<String>,
    errors: Receiver<String>,
  ) -> Result<(), std::io::Error> {
    loop {
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
      if event::poll(remaining)? {
        if let Event::Key(key) = event::read()? {
          match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char('j') => self.next_prefix(),
            KeyCode::Char('k') => self.previous_prefix(),
            KeyCode::Tab => {
              self.set_display_state(self.display_state.next());
            }
            _ => {}
          }
        }
      }
    }
  }

  fn set_display_state(&mut self, state: DisplayState) {
    self.display_state = state;
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
        res = Some(Line::with_prefix(caps[1].to_string(), caps[2].to_string()))
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
    self.error_messages.add_message(error.to_string());
  }

  pub fn get_buckets(&self) -> Vec<(usize, String)> {
    let mut vec = self
      .buckets
      .iter()
      .map(|(k, v)| (v.new_messages, k.clone()))
      .collect::<Vec<_>>();
    vec.sort_by_key(|(_, s)| s.clone());
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
      .map(|i| self.get_buckets()[i].1.clone())
  }

  pub fn get_current_messages(&mut self, count: usize) -> LinkedList<String> {
    if self.buckets.len() == 0 {
      return LinkedList::new();
    }
    // if self.messages.len
    self
      .list_state
      .selected()
      .map(|i| -> LinkedList<String> {
        let (_, prefix) = self.get_buckets()[i].clone();
        let bucket = self.buckets.get_mut(&prefix);
        bucket
          .unwrap()
          .get_messages(count)
          .iter()
          .map(|l| l.message.clone())
          .collect()
      })
      .unwrap_or(LinkedList::new())
  }
}
