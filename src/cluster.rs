use crate::builds::BuildCount;

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
pub struct Node {
  pub label: String,
  pub children: Vec<Node>,
  pub value: BuildCount,
  pub total: BuildCount,
}

impl Node {
  pub fn new(label: String, value: BuildCount, total: BuildCount) -> Node {
    Node {
      label,
      children: vec![],
      value,
      total,
    }
  }

  pub fn match_key(&self, build: &str) -> usize {
    // don't need to split into vec to compare slices
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
    // don't need to create vec to slice here either
    let buildings: Vec<&str> = self.label.split(",").collect();
    let current_node_label = &buildings[0..idx];
    let new_node_label = &buildings[idx..];

    let mut new_node = Node::new(
      new_node_label.join(","),
      self.value.clone(),
      self.total.clone(),
    );
    swap(&mut new_node.children, &mut self.children);

    self.children.push(new_node);
    self.children.sort_by(|a, b| b.total.total.cmp(&a.total.total));

    self.label = current_node_label.join(",");
    self.value.reset();
  }

  pub fn walk(&mut self, build_fragment: &str, count: &BuildCount) {
    let mut inserted = false;
    for child in &mut self.children {
      if child.label == build_fragment {
        child.total.add(&count);
        child.value.add(&count);
        self.total.add(&count);

        inserted = true;
        break;
      }

      let match_length = child.match_key(&build_fragment);
      if match_length == 0 {
        continue;
      }

      let node_build_length = child.label.split(",").collect::<Vec<&str>>().len();

      if match_length == node_build_length {
        // do I need to create a vec here?
        let buildings: Vec<&str> = build_fragment.split(",").collect();
        let next_fragment = buildings[match_length..].join(",");

        if child.children.len() != 0 {
          child.walk(&next_fragment, count);
        } else {
          let new_node = Node::new(next_fragment, count.clone(), count.clone());
          child.children.push(new_node);
          child.children.sort_by(|a, b| b.total.total.cmp(&a.total.total));
          child.total.add(&count);
        }
        self.total.add(&count);

        inserted = true;
        break;
      }

      if match_length < node_build_length {
        child.split_at(match_length);

        // do I need to create a vec here?
        let buildings: Vec<&str> = build_fragment.split(",").collect();
        if buildings.len() > match_length {
          let remaining_fragment = buildings[match_length..].join(",");
          let new_node = Node::new(remaining_fragment, count.clone(), count.clone());
          child.children.push(new_node);
          child.children.sort_by(|a, b| b.total.total.cmp(&a.total.total));
        } else {
          child.value = count.clone();
        }
        child.total.add(&count);
        self.total.add(&count);

        inserted = true;
        break;
      }

      if match_length > node_build_length {
        unreachable!("match length cannot be larger than node label");
      }
    }

    if !inserted {
      // node as &str instead of String?
      let new_node = Node::new(build_fragment.to_string(), count.clone(), count.clone());
      self.children.push(new_node);
      self.children.sort_by(|a, b| b.total.total.cmp(&a.total.total));
      self.total.add(&count);
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
