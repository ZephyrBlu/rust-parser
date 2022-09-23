use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize)]
pub struct Index {
  pub entries: HashMap<String, Vec<u32>>,
}

impl<'a> Index {
  pub fn new() -> Index {
    Index {
      entries: HashMap::new(),
    }
  }

  pub fn add(&'a mut self, value: String, id: u32) {
    if let Some(references) = self.entries.get_mut(&value) {
      // TODO: duplicate values could be an issue?
      references.push(id);
    } else {
      self.entries.insert(value, vec![id]);
    }
  }

  // pub fn search(&self, value: &'a str) -> Option<&Vec<u32>> {
  //   if let Some(references) = self.entries.get(value) {
  //     Some(references)
  //   } else {
  //     None
  //   }
  // }
}
