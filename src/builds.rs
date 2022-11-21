use core::ops::Range;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::str;

use crate::cluster::{Cluster, ClusterBuild, RadixTree};

use serde::Serialize;

#[derive(Serialize)]
pub struct BuildCount {
  total: u16,
  wins: u16,
  losses: u16,
}

pub struct Builds {
  pub builds: HashMap<String, BuildCount>,
  tokens: HashMap<String, u32>,
  cached_token_probability: HashMap<String, f32>,
  pub probability: HashMap<String, f32>,
  build_token_paths: Vec<(String, f32, u8)>,
  pub build_token_path_mappings: HashMap<String, (String, Vec<String>, f32, u8)>,
  pub token_paths: Vec<(String, f32, u8)>,
  pub skipped_builds: Vec<String>,
  pub build_comparison_information: HashMap<String, f32>,
  pub build_clusters: HashMap<String, Cluster>,
  pub build_tree: HashMap<String, RadixTree>,
  pub raw_build_tree: HashMap<String, RadixTree>,
}

const MAX_TOKEN_SIZE: usize = 5;
const SECTION_SEPARATOR: &str = "__";
const TOKEN_SEPARATOR: char = ':';
const TOKEN_TERMINATOR: &str = "NONE";
const BUILDING_SEPARATOR: &str = ",";
const BUILDING_SEPARATOR_CHAR: char = ',';
const BUILD_SEPARATOR: &str = "--";

const MAX_COMPARISON_DIFF: f32 = 15.0;

impl Builds {
  pub fn new() -> Builds {
    Builds {
      builds: HashMap::new(),
      tokens: HashMap::new(),
      cached_token_probability: HashMap::new(),
      probability: HashMap::new(),
      build_token_paths: vec![],
      build_token_path_mappings: HashMap::new(),
      token_paths: vec![],
      skipped_builds: vec![],
      build_comparison_information: HashMap::new(),
      build_clusters: HashMap::new(),
      build_tree: HashMap::new(),
      raw_build_tree: HashMap::new(),
    }
  }

  pub fn generate_tokens(&mut self, build: &Vec<String>, win: bool, token_prefix: String) {
    self.builds
      .entry(format!(
        "{token_prefix}{SECTION_SEPARATOR}{}",
        build.join(BUILDING_SEPARATOR)
      ))
      .and_modify(|build_count| {
        build_count.total += 1;
        if win {
          build_count.wins += 1;
        } else {
          build_count.losses += 1;
        }
      })
      .or_insert_with(|| {
        let mut build = BuildCount {
          total: 1,
          wins: 0,
          losses: 0,
        };

        if win {
          build.wins += 1;
        } else {
          build.losses += 1;
        }

        build
      });

    for i in 0..build.len() {
      if i > 10 {
        break;
      }

      for window_size in 1..MAX_TOKEN_SIZE + 1 {
        let tokens = &build[i..i + window_size];
        let mut current_token = tokens[0].clone();
        let mut next_token = TOKEN_TERMINATOR;

        if tokens.len() > 1 && tokens.len() != build.len() {
          current_token = tokens[..tokens.len() - 1].join(BUILDING_SEPARATOR);
          next_token = &tokens[tokens.len() - 1];
        }

        let identifier_token = format!("{token_prefix}{SECTION_SEPARATOR}{current_token}{SECTION_SEPARATOR}{next_token}");
        self.tokens
          .entry(identifier_token)
          .and_modify(|count| *count += 1)
          .or_insert(1);

        if i + window_size >= build.len() || i + window_size >= 10 {
          break;
        }
      }
    }
  }

  pub fn generate_matchup_build_trees(&mut self) {
    let mut skipped = 0;
    for (raw_build, build_count) in &self.builds {
      if build_count.total < 10 {
        skipped += 1;
      }

      let deconstructed_build: Vec<&str> = raw_build.split(SECTION_SEPARATOR).collect();
      let matchup = deconstructed_build[0];
      let build = deconstructed_build[1];

      self.raw_build_tree
        .entry(matchup.to_string())
        .and_modify(|matchup_tree| matchup_tree.insert(build, build_count.total))
        .or_insert(RadixTree::from(build, build_count.total));
    }

    for (_, tree) in &mut self.raw_build_tree {
      tree.prune(10);
    }

    println!("{:?} builds with less than 10 occurrences", skipped);
  }

  pub fn generate_token_distributions(&mut self) {
    let mut token_totals: HashMap<String, u32> = HashMap::new();
    for (key, token_count) in &self.tokens {
      let values: Vec<&str> = key.split(SECTION_SEPARATOR).collect();
      let prefix = values[0];
      let current_token = values[1];
      let next_token = values[2];

      let current_token_identifier = if next_token == TOKEN_TERMINATOR {
        format!("{prefix}{SECTION_SEPARATOR}{TOKEN_TERMINATOR}")
      } else {
        format!("{prefix}{SECTION_SEPARATOR}{current_token}")
      };

      token_totals
        .entry(current_token_identifier)
        .and_modify(|count| *count += token_count)
        .or_insert(*token_count);
    }

    for (key, count) in &self.tokens {
      let values: Vec<&str> = key.split(SECTION_SEPARATOR).collect();
      let prefix = values[0];
      let current_token = values[1];
      let next_token = values[2];

      let current_token_identifier = if next_token == TOKEN_TERMINATOR {
        format!("{prefix}{SECTION_SEPARATOR}{TOKEN_TERMINATOR}")
      } else {
        format!("{prefix}{SECTION_SEPARATOR}{current_token}")
      };

      let current_token_total = match token_totals.get(&current_token_identifier) {
        Some(total) => total,
        None => panic!("Couldn't find total for current token: {:?}", current_token),
      };

      // not enough occurrences to generate a meaningful probability
      if *current_token_total < 10 {
        continue;
      }

      let probability = *count as f32 / *current_token_total as f32;
      //  println!("token probability\n{:?}\n{:?}\n{:?} {:?} {:?}\n", key, current_token_identifier, count, current_token_total, probability);
      self.probability.insert(key.to_string(), probability);
    }
  }

  pub fn generate_token_paths(&mut self, build: &Vec<String>, build_prefix: String) {
    self.generate_next_path(String::new(), 0, 1.0, build, build_prefix.as_str(), 0);

    // sort by token path probabilities
    self.build_token_paths.sort_by(|a, b| {
      a.1.partial_cmp(&b.1)
        .expect("path probabilities should be floats")
    });

    // push highest probability path to global token paths
    if let Some(calculated_path) = self.build_token_paths.last() {
      self.token_paths.push(calculated_path.to_owned());
    } else {
      self.skipped_builds.push(build.join(BUILDING_SEPARATOR));
    }

    // clear build token paths for next build
    self.build_token_paths.clear();
  }

  fn generate_next_path<'path>(
    &mut self,
    current_path: String,
    current_path_length: usize,
    path_probability: f32,
    build: &'path Vec<String>,
    build_prefix: &str,
    build_index: usize,
  ) {
    for token_window in 1..MAX_TOKEN_SIZE + 1 {
      // base case for recursion
      if build_index + token_window > build.len() {
        // ensure we have a path the same size as our original build
        // paths may be shorter if the last token was skipped due to low probability
        if current_path_length == build.len() {
          let token_path = format!("{build_prefix}{SECTION_SEPARATOR}{current_path}");
          self.build_token_paths.push((
            token_path,
            path_probability,
            current_path_length as u8,
          ));
        }
        break;
      }

      let tokens = &build[build_index..build_index + token_window];

      // assume only 1 token is present
      let mut current_token = tokens[0].clone();
      let mut next_token = TOKEN_TERMINATOR;

      // if more than 1 token, update values
      // check against build length since token could encompass entire build if build < max token size
      if tokens.len() > 1 && tokens.len() != build.len() {
        current_token = tokens[..tokens.len() - 1].join(BUILDING_SEPARATOR);
        next_token = &tokens[tokens.len() - 1];
      }

      let mut next_path_probability = path_probability.clone();
      let identifier_token = format!(
        "{build_prefix}{SECTION_SEPARATOR}{current_token}{SECTION_SEPARATOR}{next_token}"
      );

      // if we don't have a conditional probability for the tokens there was < 10 occurrences
      if !self.probability.contains_key(&identifier_token) {
        continue;
      }

      // check to see if we've previously computed the probability of this building sequence
      if let Some(token_sequence_probability) =
        self.cached_token_probability.get(&identifier_token)
      {
        next_path_probability *= token_sequence_probability;
      } else {
        let mut token_sequence_probability = path_probability.clone();
        let mut token_fragment_window = tokens.len();

        // generate fragments of the current token backwards from the full token
        // if we find a token whose sequence we've already computed we can use that value and exit early
        for i in 0..tokens.len() {
          token_fragment_window -= i;

          // assume unigram. e.g. only 1 token is present
          let mut current_token_fragment = tokens[0].clone();
          let mut next_token_fragment = TOKEN_TERMINATOR;

          // if more than unigram, update values
          if token_fragment_window > 1 && token_fragment_window != build.len() {
            current_token_fragment =
              tokens[..token_fragment_window - 1].join(BUILDING_SEPARATOR);
            next_token_fragment = &tokens[token_fragment_window - 1];
          }

          let identifier_token_fragment = format!("{build_prefix}{SECTION_SEPARATOR}{current_token_fragment}{SECTION_SEPARATOR}{next_token_fragment}");

          // if we find a subsequence that has already been computed, use the cached value and finish the computation
          if let Some(token_fragment_sequence_probability) = self
            .cached_token_probability
            .get(&identifier_token_fragment)
          {
            token_sequence_probability *= token_fragment_sequence_probability;
            break;
          }

          // get the probability for the current fragment and add it to the sequence probability
          // this should always exist because otherwise we would have already bailed from generating the path
          match self.probability.get(&identifier_token_fragment) {
            Some(token_fragment_probability) => {
              token_sequence_probability *= token_fragment_probability
            }
            None => panic!(
              "Couldn't find fragment probability on iteration {:?} {:?} {:?}",
              i, current_token, identifier_token_fragment
            ),
          }
        }

        // add the current building sequence probability to the cache
        self.cached_token_probability
          .insert(identifier_token, token_sequence_probability);
        next_path_probability *= token_sequence_probability;
      }

      let mut next_path = current_path.clone();
      if next_path != "" {
        next_path.push(TOKEN_SEPARATOR);
      }
      next_path.push_str(tokens.join(BUILDING_SEPARATOR).as_str());
      let next_path_length = current_path_length + tokens.len();

      self.generate_next_path(
        next_path,
        next_path_length,
        next_path_probability,
        build,
        build_prefix,
        build_index + token_window,
      );
    }
  }

  // https://github.com/python/cpython/blob/c6b84a727c9299f24edbab4105ce47e9f2bae199/Lib/difflib.py#L305
  fn find_longest_match<S: Into<String> + Debug>(
    build: &Vec<S>,
    other_build: &Vec<S>,
    build_low: u8,
    build_high: u8,
    other_build_low: u8,
    other_build_high: u8,
  ) -> (u8, u8, u8)
  where
    S: Eq + Hash,
  {
    let mut other_build_mapping: HashMap<&S, Vec<u8>> = HashMap::new();
    for (index, building) in other_build.iter().enumerate() {
      other_build_mapping
        .entry(building)
        .and_modify(|indexes| indexes.push(index as u8))
        .or_insert(vec![index as u8]);
    }

    let (mut best_match_lower_bound, mut best_match_upper_bound, mut best_match_size) =
      (build_low, other_build_low, 0);

    let mut match_sizes: HashMap<i8, i8> = HashMap::new();
    for building_index in build_low..build_high {
      let mut new_match_sizes: HashMap<i8, i8> = HashMap::new();
      if let Some(building_match_indexes) =
        other_build_mapping.get(&build[building_index as usize])
      {
        for other_build_index in building_match_indexes {
          if *other_build_index < other_build_low {
            continue;
          }

          if *other_build_index >= other_build_high {
            break;
          }

          let size_lookup_index: i8 = *other_build_index as i8 - 1;
          let new_match_size: i8 = match match_sizes.get(&size_lookup_index) {
            Some(match_length) => *match_length + 1,
            None => 1,
          };
          new_match_sizes.insert(*other_build_index as i8, new_match_size);

          if new_match_size > best_match_size {
            (
              best_match_lower_bound,
              best_match_upper_bound,
              best_match_size,
            ) = (
              (building_index + 1) - new_match_size as u8,
              (other_build_index + 1) - new_match_size as u8,
              new_match_size,
            );
          }
        }
      }
      match_sizes = new_match_sizes;
    }

    while (best_match_lower_bound > build_low)
      && (best_match_upper_bound > other_build_low)
      && (build[(best_match_lower_bound - 1) as usize]
        == other_build[(best_match_upper_bound - 1) as usize])
    {
      (
        best_match_lower_bound,
        best_match_upper_bound,
        best_match_size,
      ) = (
        best_match_lower_bound - 1,
        best_match_upper_bound - 1,
        best_match_size + 1,
      );
    }

    while ((best_match_lower_bound + best_match_size as u8) < build_high)
      && ((best_match_upper_bound + best_match_size as u8) > other_build_high)
      && (build[(best_match_lower_bound + best_match_size as u8) as usize]
        == other_build[(best_match_upper_bound + best_match_size as u8) as usize])
    {
      best_match_size += 1;
    }

    (
      best_match_lower_bound,
      best_match_upper_bound,
      best_match_size as u8,
    )
  }

  // https://github.com/python/cpython/blob/c6b84a727c9299f24edbab4105ce47e9f2bae199/Lib/difflib.py#L421
  fn get_matching_blocks<S: Into<String> + Eq + Hash + Debug>(
    build: &Vec<S>,
    other_build: &Vec<S>,
  ) -> Vec<(u8, u8, u8)> {
    let mut queue: Vec<(u8, u8, u8, u8)> =
      vec![(0, build.len() as u8, 0, other_build.len() as u8)];
    let mut matching_blocks = vec![];

    while queue.len() != 0 {
      let (build_low_index, build_high_index, other_build_low_index, other_build_high_index) =
        queue.pop().unwrap();

      let longest_match = Builds::find_longest_match(
        build,
        other_build,
        build_low_index,
        build_high_index,
        other_build_low_index,
        other_build_high_index,
      );
      let (build_match_index, other_build_match_index, match_length) = longest_match;

      if match_length != 0 {
        matching_blocks.push(longest_match);
        if build_low_index < build_match_index
          && other_build_low_index < other_build_match_index
        {
          queue.push((
            build_low_index,
            build_match_index,
            other_build_low_index,
            other_build_match_index,
          ));
        }

        if (build_match_index + match_length) < build_high_index
          && (other_build_match_index + match_length) < other_build_high_index
        {
          queue.push((
            build_match_index + match_length,
            build_high_index,
            other_build_match_index + match_length,
            other_build_high_index,
          ));
        }
      }
    }
    matching_blocks.sort();

    let (
      mut previous_build_match_index,
      mut previous_other_build_match_index,
      mut previous_match_length,
    ) = (0, 0, 0);
    let mut non_adjacent = vec![];
    for (build_match_index, other_build_match_index, match_length) in &matching_blocks {
      if (previous_build_match_index + previous_match_length) == *build_match_index
        && (previous_other_build_match_index + previous_match_length)
          == *other_build_match_index
      {
        previous_match_length += match_length;
      } else {
        if previous_match_length != 0 {
          non_adjacent.push((
            previous_build_match_index,
            previous_other_build_match_index,
            previous_match_length,
          ));
        }
        (
          previous_build_match_index,
          previous_other_build_match_index,
          previous_match_length,
        ) = (*build_match_index, *other_build_match_index, *match_length)
      }
    }

    if previous_match_length != 0 {
      non_adjacent.push((
        previous_build_match_index,
        previous_other_build_match_index,
        previous_match_length,
      ));
    }

    non_adjacent
  }

  pub fn compare_builds(&mut self) {
    let mut missing_buildings: HashMap<String, u8> = HashMap::new();
    for (joined_build, _) in &self.builds {
      for (joined_other_build, _) in &self.builds {
        // skip when we encounter the same game
        if joined_build == joined_other_build {
          continue;
        }

        let build_prefix = joined_build.split(SECTION_SEPARATOR).collect::<Vec<&str>>()[0];
        let other_build_prefix = joined_other_build
          .split(SECTION_SEPARATOR)
          .collect::<Vec<&str>>()[0];

        // only generate comparisons for builds from the same race and same matchup
        if build_prefix != other_build_prefix {
          continue;
        }

        let build: Vec<String> =
          joined_build.split(SECTION_SEPARATOR).collect::<Vec<&str>>()[1]
            .split(BUILDING_SEPARATOR)
            .map(|s| s.to_string())
            .collect();
        let other_build: Vec<String> = joined_other_build
          .split(SECTION_SEPARATOR)
          .collect::<Vec<&str>>()[1]
          .split(BUILDING_SEPARATOR)
          .map(|s| s.to_string())
          .collect();

        // if the builds being compared are the same, there is no difference in information
        if joined_build == joined_other_build {
          continue;
        }

        let joined_buildings =
          joined_build.split(SECTION_SEPARATOR).collect::<Vec<&str>>()[1];
        let joined_other_buildings = joined_other_build
          .split(SECTION_SEPARATOR)
          .collect::<Vec<&str>>()[1];
        let mut comparison_builds = [
          format!("{build_prefix}{SECTION_SEPARATOR}{joined_buildings}"),
          format!("{other_build_prefix}{SECTION_SEPARATOR}{joined_other_buildings}"),
        ];
        comparison_builds.sort();
        let build_comparison_identifier = comparison_builds.join(BUILD_SEPARATOR);

        // we already have information on this build comparison
        if self
          .build_comparison_information
          .contains_key(&build_comparison_identifier)
        {
          continue;
        }

        let mut build_position: u8 = 0;
        let mut build_missing_ranges: Vec<Range<u8>> = vec![];

        let mut other_build_position: u8 = 0;
        let mut other_build_missing_ranges: Vec<Range<u8>> = vec![];

        for matching_block in Builds::get_matching_blocks(&build, &other_build) {
          // create range from previous position to start of new matching block
          build_missing_ranges.push(build_position..matching_block.0);
          other_build_missing_ranges.push(other_build_position..matching_block.1);

          build_position += matching_block.2;
          other_build_position += matching_block.2;

          if build_position >= 10 || other_build_position >= 10 {
            break;
          }
        }

        build_missing_ranges.push(build_position..build.len() as u8);
        other_build_missing_ranges.push(other_build_position..other_build.len() as u8);

        for missing_range in build_missing_ranges {
          for idx in missing_range {
            missing_buildings
              .entry(build[idx as usize].clone())
              .and_modify(|position| *position = min(idx, *position))
              .or_insert(idx);
          }
        }

        for missing_range in other_build_missing_ranges {
          for idx in missing_range {
            missing_buildings
              .entry(other_build[idx as usize].clone())
              .and_modify(|position| *position = min(idx, *position))
              .or_insert(idx);
          }
        }

        // ---

        let mut match_information_difference = 0.0;

        for (building, position) in &missing_buildings {
          let token_identifier = format!("{build_prefix}{SECTION_SEPARATOR}{building}{SECTION_SEPARATOR}{TOKEN_TERMINATOR}");

          if !self.probability.contains_key(&token_identifier) {
            continue;
          }

          let building_information = -self.probability[&token_identifier].log2();
          let position_multiplier = if *position >= 10 {
            0.0
          } else {
            2.0 * ((10 - position) as f32 / 10.0)
          };
          let tf_idf = building_information * position_multiplier;
          match_information_difference += tf_idf;
        }

        let rounded_information_difference = (match_information_difference * 100.0).round() / 100.0;
        self.build_comparison_information.insert(build_comparison_identifier, rounded_information_difference);

        missing_buildings.clear();
      }
    }
  }

  pub fn generate_clusters(&mut self) {
    for (build_comparison, _) in &self.build_comparison_information {
      let comparison_builds: Vec<&str> = build_comparison.split(BUILD_SEPARATOR).collect();

      if !self.build_clusters.contains_key(comparison_builds[0]) {
        let build_count = &self.builds[comparison_builds[0]];
        self.build_clusters.insert(
          comparison_builds[0].to_string(),
          Cluster {
              build: ClusterBuild {
                build: comparison_builds[0].to_string(),
                total: build_count.total,
                wins: build_count.wins,
                losses: build_count.losses,
                diff: 0.0,
              },
              cluster: vec![],
              matchup: String::new(),
              total: build_count.total,
              wins: build_count.wins,
              losses: build_count.losses,
              tree: RadixTree::new(),
            },
        );
      }

      if !self.build_clusters.contains_key(comparison_builds[1]) {
        let build_count = &self.builds[comparison_builds[1]];
        self.build_clusters.insert(
          comparison_builds[1].to_string(),
          Cluster {
              build: ClusterBuild {
                build: comparison_builds[1].to_string(),
                total: build_count.total,
                wins: build_count.wins,
                losses: build_count.losses,
                diff: 0.0,
              },
              cluster: vec![],
              total: build_count.total,
              wins: build_count.wins,
              losses: build_count.losses,
              matchup: String::new(),
              tree: RadixTree::new(),
            },
        );
      }
    }

    let mut all_cluster_comparisons: Vec<(String, String, f32)> = vec![];
    let mut optimal_cluster_comparisons: Vec<(String, String, f32)> = vec![];

    loop {
      let mut seen_clusters: HashSet<&String> = HashSet::new();
      for (cluster, _) in &self.build_clusters {
        for (other_cluster, _) in &self.build_clusters {
          let cluster_comparison_identifier =
            format!("{cluster}{BUILD_SEPARATOR}{other_cluster}");

          let cross_cluster_diff = match &self
            .build_comparison_information
            .get(&cluster_comparison_identifier)
          {
            None => continue, // if there is no comparison it means these are builds from different matchups/races
            Some(diff) => *diff,
          };

          if *cross_cluster_diff < MAX_COMPARISON_DIFF {
            all_cluster_comparisons.push((
              cluster.clone(),
              other_cluster.clone(),
              *cross_cluster_diff,
            ));
          }
        }
      }

      all_cluster_comparisons.sort_by(|a, b|
        a.2.partial_cmp(&b.2).expect("cluster comparisons should be floats")
      );
      for (cluster, other_cluster, cross_cluster_diff) in &all_cluster_comparisons {
        if !seen_clusters.contains(cluster) && !seen_clusters.contains(other_cluster) {
          optimal_cluster_comparisons.push((
            cluster.to_string(),
            other_cluster.to_string(),
            *cross_cluster_diff,
          ));
          seen_clusters.insert(cluster);
          seen_clusters.insert(other_cluster);
        }
      }

      let mut completed = true;
      for (build, other_build, _) in &optimal_cluster_comparisons {
        let mut cluster_complete_linkage = true;

        for clustered_build in &self.build_clusters[build].cluster {
          let mut comparison_builds = [other_build, clustered_build.build.as_str()];
          comparison_builds.sort();
          let build_comparison_identifier = format!(
            "{}{BUILD_SEPARATOR}{}",
            comparison_builds[0], comparison_builds[1]
          );

          let cross_cluster_diff = &self.build_comparison_information[&build_comparison_identifier];
          if *cross_cluster_diff > MAX_COMPARISON_DIFF {
            cluster_complete_linkage = false;
            break;
          }
        }

        for other_clustered_build in &self.build_clusters[other_build].cluster {
          let mut comparison_builds = [build, other_clustered_build.build.as_str()];
          comparison_builds.sort();
          let build_comparison_identifier = format!(
            "{}{BUILD_SEPARATOR}{}",
            comparison_builds[0],
            comparison_builds[1],
          );

          let cross_cluster_diff = &self.build_comparison_information[&build_comparison_identifier];
          if *cross_cluster_diff > MAX_COMPARISON_DIFF {
            cluster_complete_linkage = false;
            break;
          }
        }

        if !cluster_complete_linkage {
          continue;
        }

        let build_count = self.builds[build].total;
        let other_build_count = self.builds[other_build].total;

        let max_build;
        let min_build;
        if build_count > other_build_count {
          max_build = build;
          min_build = other_build;
        } else {
          max_build = other_build;
          min_build = build;
        }

        let mut min_build_cluster = self.build_clusters[min_build].clone();
        if let Some(max_cluster) = self.build_clusters.get_mut(max_build) {
          // update min cluster diffs to max cluster
          for build in &mut min_build_cluster.cluster {
            let mut comparison_builds = [max_build, &build.build];
            comparison_builds.sort();

            let build_comparison_identifier = format!(
              "{}{BUILD_SEPARATOR}{}",
              comparison_builds[0],
              comparison_builds[1],
            );
            let cross_cluster_diff = &self.build_comparison_information[&build_comparison_identifier];
            build.diff = *cross_cluster_diff;

            max_cluster.total += build.total;
            max_cluster.wins += build.wins;
            max_cluster.losses += build.losses;
          }

          max_cluster.cluster.extend(min_build_cluster.cluster);

          let mut comparison_builds = [max_build, min_build];
          comparison_builds.sort();

          let build_comparison_identifier = format!(
            "{}{BUILD_SEPARATOR}{}",
            comparison_builds[0],
            comparison_builds[1],
          );
          let cross_cluster_diff = &self.build_comparison_information[&build_comparison_identifier];

          max_cluster.cluster.push(ClusterBuild {
            build: min_build.to_string(),
            total: min_build_cluster.build.total,
            wins: min_build_cluster.build.wins,
            losses: min_build_cluster.build.losses,
            diff: *cross_cluster_diff,
          });
        }
        self.build_clusters.remove(&min_build.to_string());

        completed = false;
      }

      if completed {
        let mut builds: Vec<Cluster> = vec![];

        for (_, cluster) in &mut self.build_clusters {
          builds.push(cluster.clone());

          cluster.cluster.sort_by(|a, b|
            a.diff.partial_cmp(&b.diff).expect("build diff should be a float")
          );

          let deconstructed_root_build: Vec<&str> = cluster.build.build.split(SECTION_SEPARATOR).collect();
          let matchup = deconstructed_root_build[0];
          let root_buildings = deconstructed_root_build[1];
          cluster.matchup = matchup.to_string();
          cluster.tree.insert(&root_buildings.to_string(), cluster.build.total);

          for (idx, build) in cluster.cluster.iter().enumerate() {
            if idx >= 25 {
              break;
            }
            let deconstructed_build: Vec<&str> = build.build.split(SECTION_SEPARATOR).collect();
            let buildings = deconstructed_build[1];
            cluster.tree.insert(&buildings.to_string(), build.total);
          }
          // Builds::generate_build_tree(cluster);
        }

        builds.sort_by(|a, b|
          b.build.total.cmp(&a.build.total)
        );

        let mut matchup_clusters: HashMap<&str, u8> = HashMap::new();
        for cluster in builds.iter() {
          let deconstructed_root_build: Vec<&str> = cluster.build.build.split(SECTION_SEPARATOR).collect();
          let matchup = deconstructed_root_build[0];
          let root_buildings = deconstructed_root_build[1];

          if let Some(count) = matchup_clusters.get(&matchup) {
            if *count >= 25 {
              continue;
            }
          }

          self.build_tree
            .entry(matchup.to_string())
            .and_modify(|tree| tree.insert(&root_buildings.to_string(), cluster.build.total))
            .or_insert(RadixTree::from(&root_buildings.to_string(), cluster.build.total));
          matchup_clusters
            .entry(matchup)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        }

        break;
      }

      all_cluster_comparisons.clear();
      optimal_cluster_comparisons.clear();
    }
  }
}
