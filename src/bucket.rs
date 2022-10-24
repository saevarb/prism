use log::debug;

use crate::app::Line;

#[derive(Clone, Debug)]
pub struct Bucket {
  messages: Vec<Line>,
  pub new_messages: usize,
  pub new_errors: usize,
  pub scroll: Option<usize>,
}

impl Bucket {
  pub fn new() -> Bucket {
    Bucket {
      messages: Default::default(),
      new_messages: 0,
      new_errors: 0,
      scroll: None,
    }
  }

  pub fn from_messages(messages: Vec<Line>) -> Bucket {
    Bucket {
      messages: messages.into_iter().collect(),
      new_messages: 0,
      new_errors: 0,
      scroll: None,
    }
  }

  pub fn get_all_messages(&self) -> &Vec<Line> {
    &self.messages
  }

  pub fn add_message(&mut self, message: Line) {
    self.new_errors += if message.has_error { 1 } else { 0 };
    self.new_messages += 1;
    self.messages.push(message);
  }

  pub fn get_older(&self, height: usize) -> usize {
    if let Some(scroll) = self.scroll {
      debug!(
        "get_older: scroll = {}, height = {}, msgs: {}",
        scroll,
        height,
        self.messages.len()
      );
      self.messages.len().max(scroll + height) - scroll - height
    } else {
      0
    }
  }

  pub fn get_messages(&mut self, count: usize) -> Vec<Line> {
    self.new_messages = 0;
    self.new_errors = 0;

    let skip = self
      .scroll
      .unwrap_or((self.messages.len() as i32 - count as i32).max(0) as usize);

    debug!("scroll: {:?}", self.scroll);
    debug!("skip: {}", skip);

    self
      .messages
      .iter()
      .skip(skip)
      .take(count)
      .cloned()
      .collect()
  }

  pub fn scroll_up(&mut self, height: usize) {
    if self.messages.len() < height + 1 {
      return;
    }
    self.scroll = if let Some(scroll) = self.scroll {
      if scroll == 0 {
        Some(0)
      } else {
        Some(scroll - 1)
      }
    } else {
      Some(self.messages.len() - height - 1)
    };
  }

  pub fn scroll_down(&mut self, height: usize) {
    self.scroll = if let Some(scroll) = self.scroll {
      if scroll + height >= self.messages.len() {
        None
      } else {
        Some(scroll + 1)
      }
    } else {
      None
    };
  }

  pub fn scroll_reset(&mut self) {
    self.scroll = None;
  }
}
