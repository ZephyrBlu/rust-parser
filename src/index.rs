use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize)]
pub struct Index {
  pub name: String,
  pub id_entries: HashMap<String, Vec<u32>>,
  pub hash_entries: HashMap<String, Vec<String>>,
}

impl<'a> Index {
  pub fn new(name: &str) -> Index {
    Index {
      name: name.to_string(),
      id_entries: HashMap::new(),
      hash_entries: HashMap::new(),
    }
  }

  pub fn add_id(&'a mut self, value: String, id: u32) {
    if let Some(references) = self.id_entries.get_mut(&value) {
      // TODO: duplicate values could be an issue?
      references.push(id);
    } else {
      self.id_entries.insert(value, vec![id]);
    }
  }

  pub fn add_hash(&'a mut self, value: String, hash: String) {
    if let Some(references) = self.hash_entries.get_mut(&value) {
      // TODO: duplicate values could be an issue?
      references.push(hash);
    } else {
      self.hash_entries.insert(value, vec![hash]);
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
