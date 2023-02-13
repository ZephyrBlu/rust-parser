use std::cmp::min;
use std::mem::swap;
use std::time::Instant;

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ClusterTreeNode {
  pub label: String,
  pub matchup: String,
  pub children: Vec<ClusterTreeNode>,
}

#[derive(Serialize, Clone)]
pub struct ClusterTree {
  pub terms: Vec<ClusterTreeNode>,
}

#[derive(Serialize, Clone)]
pub struct Cluster {
  pub build: ClusterBuild,
  pub matchup: String,
  pub total: u16,
  pub wins: u16,
  pub losses: u16,
  pub cluster: Vec<ClusterBuild>,
  pub tree: RadixTrie,
}

#[derive(Serialize, Clone)]
pub struct ClusterBuild {
  pub build: String,
  pub total: u16,
  pub wins: u16,
  pub losses: u16,
  pub diff: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct BuildCount {
  pub total: u16,
  pub wins: u16,
  pub losses: u16,
}

impl BuildCount {
  pub fn new() -> BuildCount {
    BuildCount {
      total: 0,
      wins: 0,
      losses: 0,
    }
  }

  pub fn add(&mut self, other_build_count: &BuildCount) {
    self.total += other_build_count.total;
    self.wins += other_build_count.wins;
    self.losses += other_build_count.losses;
  }
}

// use array of 16 instead of vec, big enough for my purposes
// can bounds check on insertion and reject new insertions if len is 16
#[derive(Serialize, Clone, Debug)]
pub struct Node {
  pub label: String,
  pub children: Vec<Node>,
  pub value: BuildCount,
}

impl Node {
  pub fn new(label: String, value: BuildCount) -> Node {
    Node {
      label,
      children: vec![],
      value,
    }
  }

  pub fn match_key(&self, build: &str) -> usize {
    let mut match_length = 0;
    let key_chars = build.as_bytes();
    let node_chars = self.label.as_bytes();

    let upper_bound = min(key_chars.len(), node_chars.len());

    // doesn't account for full match scenario as that is covered beforehand
    for idx in 0..upper_bound {
      let current_key_char = key_chars[idx];
      let current_node_char = node_chars[idx];

      if current_key_char == current_node_char {
        // 44 = comma char
        if current_key_char == 44 {
          match_length = idx;
        }
      } else {
        break;
      }
    }

    match_length
  }

  pub fn split_at(&mut self, idx: usize) {
    let current_node_label = &self.label[..idx];
    let new_node_label = &self.label[idx + 1..];

    let mut new_node = Node::new(
      new_node_label.to_string(),
      self.value.clone(),
    );
    swap(&mut new_node.children, &mut self.children);

    self.children.push(new_node);
    self.label = current_node_label.to_string();
  }

  // refactor this to use matches instead of only if statements
  pub fn walk(&mut self, build_fragment: &str, count: &BuildCount) {
    let mut inserted = false;
    for child in &mut self.children {
      if child.label == build_fragment {
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      let compare_fragment = if build_fragment.len() > child.label.len() {
        &build_fragment[..child.label.len()]
      } else {
        build_fragment
      };

      if compare_fragment == child.label {
        let next_fragment = &build_fragment[child.label.len() + 1..];

        if child.children.len() != 0 {
          child.walk(&next_fragment, count);
        } else {
          let new_node = Node::new(next_fragment.to_string(), count.clone());
          child.children.push(new_node);
          child.value.add(&count);
        }
        self.value.add(&count);

        inserted = true;
        break;
      }

      if child.label.starts_with(compare_fragment) {
        child.split_at(compare_fragment.len());

        child.value = count.clone();
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      let match_length = child.match_key(&build_fragment);
      if match_length == 0 {
        continue;
      }

      if match_length < child.label.len() {
        child.split_at(match_length);

        let remaining_fragment = build_fragment[match_length + 1..].to_string();
        let new_node = Node::new(remaining_fragment, count.clone());
        child.children.push(new_node);
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      if match_length > child.label.len() {
        unreachable!("match length cannot be larger than node label\nmatch length {:?} on:\n{:?}\n{:?}", match_length, build_fragment, child.label);
      }
    }

    if !inserted {
      let new_node = Node::new(build_fragment.to_string(), count.clone());
      self.children.push(new_node);
      self.value.add(&count);
    }
  }
}

#[derive(Serialize, Clone, Debug)]
pub struct RadixTrie {
  pub root: Node,
}

impl RadixTrie {
  pub fn new() -> RadixTrie {
    RadixTrie {
      root: Node::new(
        String::from("ROOT"),
        BuildCount::new(),
      ),
    }
  }

  pub fn from(build: &str, count: BuildCount) -> RadixTrie {
    let mut tree = RadixTrie::new();
    tree.insert(build, count);

    tree
  }

  pub fn insert(&mut self, build: &str, count: BuildCount) {
    self.root.walk(build, &count);
  }
}
