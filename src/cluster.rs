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
  pub cluster: BuildList,
  pub tree: RadixTree,
}

#[derive(Serialize, Clone)]
pub struct ClusterBuild {
  pub build: String,
  pub count: u16,
  pub diff: f32,
}

#[derive(Serialize, Clone)]
pub struct BuildList {
  pub total_count: u16,
  pub builds: Vec<ClusterBuild>,
}

#[derive(Serialize, Clone)]
pub struct Node {
  pub label: String,
  pub children: Vec<Node>,
  pub value: u16,
}

impl Node {
  pub fn new(label: String, value: u16) -> Node {
    Node {
      label,
      children: vec![],
      value,
    }
  }

  pub fn match_key(&self, build: &String) -> usize {
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

    self.label = current_node_label.join(",");
    self.value = 0;
  }

  pub fn walk(&mut self, build_fragment: &String, count: u16) {
    let mut inserted = false;
    for child in &mut self.children {
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
        }

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
        } else {
          child.value = count;
        }

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
    }
  }
}

#[derive(Serialize, Clone)]
pub struct RadixTree {
  pub root: Vec<Node>,
}

impl RadixTree {
  pub fn new() -> RadixTree {
    RadixTree {
      root: vec![],
    }
  }

  pub fn from(build: &String, count: u16) -> RadixTree {
    let mut tree = RadixTree::new();
    tree.insert(build, count);
    tree
  }

  pub fn insert(&mut self, build: &String, count: u16) {
    let mut inserted = false;
    for node in &mut self.root {
      let match_length = node.match_key(&build);
      if match_length == 0 {
        continue;
      }

      let node_build_length = node.label.split(",").collect::<Vec<&str>>().len();

      if match_length == node_build_length {
        let buildings: Vec<&str> = build.split(",").collect();
        let build_fragment = buildings[match_length..].join(",");

        if node.children.len() != 0 {
          node.walk(&build_fragment, count);
        } else {
          let new_node = Node::new(build_fragment, count);
          node.children.push(new_node);
        }

        inserted = true;
        break;
      }

      if match_length < node_build_length {
        node.split_at(match_length);

        let buildings: Vec<&str> = build.split(",").collect();
        if buildings.len() > match_length {
          let remaining_fragment = buildings[match_length..].join(",");
          let new_node = Node::new(remaining_fragment, count);
          node.children.push(new_node);
        } else {
          node.value = count;
        }

        inserted = true;
        break;
      }

      if match_length > node_build_length {
        unreachable!("match length cannot be larger than node length");
      }
    }

    if !inserted {
      let new_root = Node::new(build.to_string(), count);
      self.root.push(new_root);
    }
  }
}

