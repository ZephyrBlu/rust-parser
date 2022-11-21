use std::cmp::{min};
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
  pub tree: RadixTree,
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
pub struct Node {
  pub label: String,
  pub children: Vec<Node>,
  pub value: u16,
  pub total: u16,
}

impl Node {
  pub fn new(label: String, value: u16) -> Node {
    Node {
      label,
      children: vec![],
      value,
      total: value,
    }
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

    let mut new_node = Node::new(new_node_label.join(","), self.value);
    swap(&mut new_node.children, &mut self.children);

    self.children.push(new_node);
    self.children.sort_by(|a, b| b.total.cmp(&a.total));

    self.label = current_node_label.join(",");
    self.value = 0;
  }

  pub fn walk(&mut self, build_fragment: &str, count: u16) {
    let mut inserted = false;
    for child in &mut self.children {
      if child.label == build_fragment {
        child.total += count;
        child.value += count;

        inserted = true;
        break;
      }

      let match_length = child.match_key(&build_fragment);
      if match_length == 0 {
        continue;
      }

      let node_build_length = child.label.split(",").collect::<Vec<&str>>().len();

      if match_length == node_build_length {
        let buildings: Vec<&str> = build_fragment.split(",").collect();
        let next_fragment = buildings[match_length..].join(",");

        if child.children.len() != 0 {
          child.walk(&next_fragment, count);
        } else {
          let new_node = Node::new(next_fragment, count);
          child.children.push(new_node);
          child.children.sort_by(|a, b| b.total.cmp(&a.total));
          child.total += count;
        }
        self.total += count;

        inserted = true;
        break;
      }

      if match_length < node_build_length {
        child.split_at(match_length);

        let buildings: Vec<&str> = build_fragment.split(",").collect();
        if buildings.len() > match_length {
          let remaining_fragment = buildings[match_length..].join(",");
          let new_node = Node::new(remaining_fragment, count);
          child.children.push(new_node);
          child.children.sort_by(|a, b| b.total.cmp(&a.total));
        } else {
          child.value = count;
        }
        child.total += count;
        self.total += count;

        inserted = true;
        break;
      }

      if match_length > node_build_length {
        unreachable!("match length cannot be larger than node length");
      }
    }

    if !inserted {
      let new_node = Node::new(build_fragment.to_string(), count);
      self.children.push(new_node);
      self.children.sort_by(|a, b| b.total.cmp(&a.total));
      self.total += count;
    }
  }

  pub fn prune(&mut self, min_count: u16) -> i16 {
    let mut nodes_to_remove = vec![];
    for (idx, child) in self.children.iter_mut().enumerate() {
      if child.total < min_count {
        nodes_to_remove.push(idx);
      } else {
        let child_total = child.prune(min_count);
        if child_total == -1 {
          nodes_to_remove.push(idx);
        }
      }
    }

    // sort and reverse to remove from back first, since removal changes position
    nodes_to_remove.sort();
    nodes_to_remove.reverse();
    for idx in nodes_to_remove {
      self.children.remove(idx);
    }

    // merge if 1 child and is not a leaf node
    if self.label != "ROOT" && self.children.len() == 1 && self.value == 0 {
      let child = &self.children[0];

      self.value = child.value;
      self.label = format!("{},{}", self.label, child.label);

      // re-parent children to current node
      self.children = child.children.clone();
    }

    if self.children.len() == 0 && self.total < min_count {
      -1
    } else {
      self.total as i16
    }
  }
}

#[derive(Serialize, Clone, Debug)]
pub struct RadixTree {
  pub root: Node,
}

impl RadixTree {
  pub fn new() -> RadixTree {
    RadixTree {
      root: Node::new(String::from("ROOT"), 0),
    }
  }

  pub fn from(build: &str, count: u16) -> RadixTree {
    let mut tree = RadixTree::new();
    tree.insert(build, count);
    tree
  }

  pub fn insert(&mut self, build: &str, count: u16) {
    if build == "" {
      return;
    }
    self.root.walk(build, count);
  }

  pub fn prune(&mut self, min_count: u16) {
    self.root.prune(min_count);
  }
}
