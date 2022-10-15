use log::debug;
use std::collections::{LinkedList, VecDeque};

#[derive(Clone, Debug)]
pub struct Bucket<I> {
  messages: Vec<I>,
  pub new_messages: usize,
  pub scroll: Option<usize>,
}

impl<I: Clone> Bucket<I> {
  pub fn new() -> Bucket<I> {
    Bucket {
      messages: Default::default(),
      new_messages: 0,
      scroll: None,
    }
  }

  pub fn from_messages(messages: Vec<I>) -> Bucket<I> {
    Bucket {
      messages: messages.into_iter().collect(),
      new_messages: 0,
      scroll: None,
    }
  }

  pub fn get_all_messages(&self) -> &Vec<I> {
    &self.messages
  }

  pub fn add_message(&mut self, message: I) {
    self.messages.push(message);
    self.new_messages += 1;
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

  pub fn get_messages(&mut self, count: usize) -> Vec<I> {
    self.new_messages = 0;

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
