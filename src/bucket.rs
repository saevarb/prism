use std::collections::LinkedList;

#[derive(Clone, Debug)]
pub struct Bucket<I> {
  messages: LinkedList<I>,
  pub new_messages: usize,
}

impl<I: Clone> Bucket<I> {
  pub fn new() -> Bucket<I> {
    Bucket {
      messages: LinkedList::new(),
      new_messages: 0,
    }
  }

  pub fn from_messages(messages: Vec<I>) -> Bucket<I> {
    Bucket {
      messages: messages.into_iter().collect(),
      new_messages: 0,
    }
  }

  pub fn add_message(&mut self, message: I) {
    self.messages.push_front(message);
    self.new_messages += 1;
  }

  pub fn get_messages(&mut self, count: usize) -> LinkedList<I> {
    self.new_messages = 0;
    self.messages.iter().take(count).rev().cloned().collect()
  }
}
