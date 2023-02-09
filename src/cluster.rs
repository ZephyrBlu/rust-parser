use std::cmp::min;
use std::mem::swap;
use std::time::{Instant, Duration};

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
    let key_buildings: Vec<&str> = build.split(",").collect();
    let node_buildings: Vec<&str> = self.label.split(",").collect();

    let mut match_length = 0;
    let upper_bound = min(key_buildings.len(), node_buildings.len());
    for idx in 0..upper_bound {
      let current_key_building = key_buildings[idx];
      let current_node_building = node_buildings[idx];

      if current_key_building == current_node_building {
        // account for joining commas except if last item
        let current_match_length = if idx == upper_bound - 1 {
          current_key_building.len()
        } else {
          current_key_building.len() + 1
        };
        match_length += current_match_length;
      } else {
        break;
      }
    }

    match_length
  }

  pub fn split_at(&mut self, idx: usize) {
    let current_node_label = &self.label[..idx];
    let new_node_label = &self.label[idx..];

    let mut new_node = Node::new(
      new_node_label.to_string(),
      self.value.clone(),
    );
    swap(&mut new_node.children, &mut self.children);

    self.children.push(new_node);
    self.label = current_node_label.to_string();
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

      let compare_fragment = if build_fragment.len() > child.label.len() {
        &build_fragment[..child.label.len()]
      } else {
        build_fragment
      };

      if compare_fragment == child.label {
        let next_fragment = &build_fragment[child.label.len()..];

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

      if child.label.contains(compare_fragment) {
        let match_length = compare_fragment.len();
        child.split_at(match_length);

        child.value = count.clone();
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      // new and optimized comparisons end here

      let match_length = child.match_key(&build_fragment);
      if match_length == 0 {
        continue;
      }

      if match_length < child.label.len() {
        child.split_at(match_length);

        let remaining_fragment = build_fragment[match_length..].to_string();
        let new_node = Node::new(remaining_fragment, count.clone());
        child.children.push(new_node);
        child.value.add(&count);
        self.value.add(&count);

        inserted = true;
        break;
      }

      if match_length > child.label.len() {
        unreachable!("match length cannot be larger than node label");
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
  pub insert_time: Vec<u128>,
}

impl RadixTrie {
  pub fn new() -> RadixTrie {
    RadixTrie {
      root: Node::new(
        String::from("ROOT"),
        BuildCount::new(),
      ),
      insert_time: vec![],
    }
  }

  pub fn from(build: &str, count: BuildCount) -> RadixTrie {
    let mut tree = RadixTrie::new();
    tree.insert(build, count);

    tree
  }

  pub fn insert(&mut self, build: &str, count: BuildCount) {
    let start = Instant::now();
    self.root.walk(build, &count);
    let finish = start.elapsed();
    self.insert_time.push(finish.as_micros());
  }
}
