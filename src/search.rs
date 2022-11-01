use crate::index::Index;

use std::collections::{HashMap, HashSet};

pub struct Search {
  pub results: HashMap<String, HashSet<String>>,
}

impl Search {
  pub fn new() -> Search {
    Search {
      results: HashMap::new(),
    }
  }

  pub fn search(&mut self, term: String, indexes: &Vec<&Index>) {
    let query_key = term.split_whitespace().collect::<Vec<&str>>().join("-");

    for index in indexes {
      if let Some(references) = index.hash_entries.get(&term) {
        let results_key = format!("{}__{}", index.name, query_key);
        self.results
          .entry(results_key)
          .and_modify(|results| results.extend(references.clone()))
          .or_insert(HashSet::from_iter(references.clone()));
      }
    }
  }
}
