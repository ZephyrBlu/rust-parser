use std::cmp::min;
use std::mem::swap;

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

  pub fn reset(&mut self) {
    self.total = 0;
    self.wins = 0;
    self.losses = 0;
  }
}

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

  fn key_length(key: &String) -> usize {
    let separator_matches: Vec<&str> = key.matches(",").collect();
    separator_matches.len() + 1
  }

  pub fn match_key(&self, build: &str) -> usize {
    let key_buildings: Vec<&str> = build.split(",").collect();
    let node_buildings: Vec<&str> = self.label.split(",").collect();

    let mut match_length = 0;
    for idx in 0..min(key_buildings.len(), node_buildings.len()) {
      let current_key_building = key_buildings[idx];
      let current_node_building = node_buildings[idx];

      if current_key_building == current_node_building {
        match_length += 1;
      } else {
        break;
      }
    }

    match_length
  }

  pub fn split_at(&mut self, idx: usize) {
    let buildings: Vec<&str> = self.label.split(",").collect();
    let current_node_label = &buildings[0..idx];
    let new_node_label = &buildings[idx..];

    let mut new_node = Node::new(
      new_node_label.join(","),
      self.value.clone(),
    );
    swap(&mut new_node.children, &mut self.children);

    self.children.push(new_node);
    self.children.sort_by(|a, b| b.value.total.cmp(&a.value.total));
    self.label = current_node_label.join(",");
    self.value.reset();
  }

  pub fn walk(&mut self, build_fragment: &str, count: &BuildCount) {
    let mut inserted = false;
    for child in &mut self.children {
      if child.label == build_fragment {
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      let match_length = child.match_key(&build_fragment);
      if match_length == 0 {
        continue;
      }

      let node_build_length = Node::key_length(&child.label);
      if match_length == node_build_length {
        let buildings: Vec<&str> = build_fragment.split(",").collect();
        let next_fragment = buildings[match_length..].join(",");

        if child.children.len() != 0 {
          child.walk(&next_fragment, count);
        } else {
          let new_node = Node::new(next_fragment, count.clone());
          child.children.push(new_node);
          child.children.sort_by(|a, b| b.value.total.cmp(&a.value.total));
          child.value.add(&count);
        }
        self.value.add(&count);

        inserted = true;
        break;
      }

      if match_length < node_build_length {
        child.split_at(match_length);

        let buildings: Vec<&str> = build_fragment.split(",").collect();
        if buildings.len() > match_length {
          let remaining_fragment = buildings[match_length..].join(",");
          let new_node = Node::new(remaining_fragment, count.clone());
          child.children.push(new_node);
          child.children.sort_by(|a, b| b.value.total.cmp(&a.value.total));
        } else {
          child.value = count.clone();
        }
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      if match_length > node_build_length {
        unreachable!("match length cannot be larger than node label");
      }
    }

    if !inserted {
      let new_node = Node::new(build_fragment.to_string(), count.clone());
      self.children.push(new_node);
      self.children.sort_by(|a, b| b.value.total.cmp(&a.value.total));
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

  // use reference to build count instead of cloning for insert
  pub fn insert(&mut self, build: &str, count: BuildCount) {
    self.root.walk(build, &count);
  }
}
