use std::collections::HashMap;
use std::hash::Hash;
use std::fmt::Debug;

pub struct Builds {
  tokens: HashMap<String, u32>,
  cached_token_probability: HashMap<String, f32>,
  pub probability: HashMap<String, f32>,
  build_token_paths: Vec<(String, f32, u8)>,
  pub token_paths: Vec<(String, f32, u8)>,
  pub skipped_builds: Vec<String>,
  // cluster_comparisons: HashMap<>,
}

const MAX_TOKEN_SIZE: usize = 4;
const TOKEN_SEPARATOR: char = ':';
const TOKEN_TERMINATOR: &str = "NONE";
const BUILDING_SEPARATOR: &str = ",";
const BUILDING_SEPARATOR_CHAR: char = ',';
const MATCHUP_SEPARATOR: &str = "-";

impl Builds {
  pub fn new() -> Builds {
    Builds {
      tokens: HashMap::new(),
      cached_token_probability: HashMap::new(),
      probability: HashMap::new(),
      build_token_paths: vec![],
      token_paths: vec![],
      skipped_builds: vec![],
    }
  }

  pub fn generate_tokens(&mut self, build: &Vec<String>, token_prefix: String) {
    for i in 0..build.len() {
      for window_size in 1..MAX_TOKEN_SIZE + 1 {
        let tokens = &build[i..i + window_size];
        let mut current_token = tokens[0].clone();
        let mut next_token = TOKEN_TERMINATOR;

        if tokens.len() > 1 && tokens.len() != build.len() {
          current_token = tokens[..tokens.len() - 1].join(BUILDING_SEPARATOR);
          next_token = &tokens[tokens.len() - 1];
        }

        let identifier_token = format!("{token_prefix}__{current_token}__{next_token}");
        self.tokens.entry(identifier_token).and_modify(|count| *count += 1).or_insert(1);

        if i + window_size >= build.len() {
          break;
        }
      }
    }
  }

  pub fn generate_token_distributions(&mut self) {
    let mut token_totals: HashMap<String, u32> = HashMap::new();
    for (key, token_count) in &self.tokens {
      let values: Vec<&str> = key.split("__").collect();
      let prefix = values[0];
      let current_token = values[1];
      let next_token = values[2];

      let current_token_identifier = if next_token == TOKEN_TERMINATOR {
        format!("{prefix}__{TOKEN_TERMINATOR}")
      } else {
        format!("{prefix}__{current_token}")
      };

      token_totals
        .entry(current_token_identifier)
        .and_modify(|count| *count += token_count)
        .or_insert(*token_count);
    }

    for (key, count) in &self.tokens {
      let values: Vec<&str> = key.split("__").collect();
      let prefix = values[0];
      let current_token = values[1];
      let next_token = values[2];

      let current_token_identifier = if next_token == TOKEN_TERMINATOR {
        format!("{prefix}__{TOKEN_TERMINATOR}")
      } else {
        format!("{prefix}__{current_token}")
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
      // println!("token probability\n{:?}\n{:?}\n{:?} {:?} {:?}\n", key, current_token_identifier, count, current_token_total, probability);
      self.probability.insert(key.to_string(), probability);
    }
  }

  pub fn generate_token_paths(&mut self, build: &Vec<String>, token_prefix: String) {
    self.generate_next_path(
      String::new(),
      0,
      1.0,
      build,
      token_prefix.as_str(),
      0,
    );

    // sort by token path probabilities
    self.build_token_paths
      .sort_by(|a, b|
        a.1
          .partial_cmp(&b.1)
          .expect("path probabilities should be floats"));

    // push highest probability path to global token paths
    if let Some(calculated_path) = self.build_token_paths.last() {
      self.token_paths.push(calculated_path.to_owned());
    } else {
      self.skipped_builds.push(build.join(BUILDING_SEPARATOR));
    }

    // clear build token paths for next build
    self.build_token_paths.clear();
  }

  fn generate_next_path<'a>(
    &mut self,
    current_path: String,
    current_path_length: usize,
    path_probability: f32,
    build: &'a Vec<String>,
    token_prefix: &str,
    build_index: usize,
  ) {
    for token_window in 1..MAX_TOKEN_SIZE + 1 {
      // base case for recursion
      if build_index + token_window > build.len() {
        // ensure we have a path the same size as our original build
        // paths may be shorter if the last token was skipped due to low probability
        if current_path_length == build.len() {
          self.build_token_paths.push((current_path, path_probability, current_path_length as u8));
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
      let identifier_token = format!("{token_prefix}__{current_token}__{next_token}");

      // if we don't have a conditional probability for the tokens there was < 10 occurrences
      if !self.probability.contains_key(&identifier_token) {
        continue;
      }

      // check to see if we've previously computed the probability of this building sequence
      if let Some(token_sequence_probability) = self.cached_token_probability.get(&identifier_token) {
        next_path_probability *= token_sequence_probability;
      } else {
        let mut token_sequence_probability = path_probability.clone();
        let mut token_fragment_window = tokens.len();

        // generate fragments of the current token backwards from the full token
        // if we find a token whose sequence we've already computed we can use that value and exit early
        for i in 0..tokens.len() {
          token_fragment_window  -= i;

          // assume unigram. e.g. only 1 token is present
          let mut current_token_fragment = tokens[0].clone();
          let mut next_token_fragment = TOKEN_TERMINATOR;

          // if more than unigram, update values
          if token_fragment_window > 1 && token_fragment_window != build.len() {
            current_token_fragment = tokens[..token_fragment_window - 1].join(BUILDING_SEPARATOR);
            next_token_fragment = &tokens[token_fragment_window - 1];
          }

          let identifier_token_fragment = format!("{token_prefix}__{current_token_fragment}__{next_token_fragment}");

          // if we find a subsequence that has already been computed, use the cached value and finish the computation
          if let Some(token_fragment_sequence_probability) = self.cached_token_probability.get(&identifier_token_fragment) {
            token_sequence_probability *= token_fragment_sequence_probability;
            break;
          }

          // get the probability for the current fragment and add it to the sequence probability
          // this should always exist because otherwise we would have already bailed from generating the path
          match self.probability.get(&identifier_token_fragment) {
            Some(token_fragment_probability) => token_sequence_probability *= token_fragment_probability,
            None => panic!("Couldn't find fragment probability on iteration {:?} {:?} {:?}", i, current_token, identifier_token_fragment),
          }
        }

        // add the current building sequence probability to the cache
        self.cached_token_probability.insert(identifier_token, token_sequence_probability);
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
        token_prefix,
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
  ) -> (u8, u8, u8) where S: Eq + Hash {
    let mut other_build_mapping: HashMap<&S, Vec<u8>> = HashMap::new();
    for (index, building) in other_build.iter().enumerate() {
      other_build_mapping
        .entry(building)
        .and_modify(|indexes| indexes.push(index as u8))
        .or_insert(vec![index as u8]);
    }

    let (
      mut best_match_lower_bound,
      mut best_match_upper_bound,
      mut best_match_size,
    ) = (build_low, other_build_low, 0);

    let mut match_sizes: HashMap<i8, i8> = HashMap::new();
    for building_index in build_low..build_high {
      let mut new_match_sizes: HashMap<i8, i8> = HashMap::new();
      if let Some(building_match_indexes) = other_build_mapping.get(&build[building_index as usize]) {
        println!("\nbuilding match indexes {:?} {:?}", &build[building_index as usize], building_match_indexes);
        for other_build_index in building_match_indexes {
          println!("checking other build indexes {:?} {:?} {:?}", other_build_index, build_low, build_high);
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
          new_match_sizes.insert(*other_build_index as i8,  new_match_size);

          println!("match size comparison {:?}, {:?} {:?}, {:?} {:?}", size_lookup_index, building_index, other_build_index, new_match_size, best_match_size);

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
            println!("updated match {:?} {:?} {:?}", best_match_lower_bound, best_match_upper_bound, best_match_size);
          }
        }
      }
      match_sizes = new_match_sizes;
    }

    if best_match_lower_bound > 0 && best_match_upper_bound > 0 {
      println!("matching stuff {:?} > {:?}, {:?} > {:?}, {:?} == {:?}\n", best_match_lower_bound, build_low, best_match_upper_bound, other_build_low, build[(best_match_lower_bound - 1) as usize], other_build[(best_match_upper_bound - 1) as usize]);
    }

    while
      (best_match_lower_bound > build_low) &&
      (best_match_upper_bound > other_build_low) &&
      (build[(best_match_lower_bound - 1) as usize] == other_build[(best_match_upper_bound - 1) as usize])
    {
      println!("looping");
      println!("matching stuff {:?} > {:?}, {:?} > {:?}, {:?} == {:?}", best_match_lower_bound, build_low, best_match_upper_bound, other_build_low, build[(best_match_lower_bound - 1) as usize], other_build[(best_match_upper_bound - 1) as usize]);
      (
        best_match_lower_bound,
        best_match_upper_bound,
        best_match_size,
      ) = (
        best_match_lower_bound - 1,
        best_match_upper_bound - 1,
        best_match_size + 1,
      );
      println!("updated {:?} {:?} {:?}\n", best_match_lower_bound, best_match_upper_bound, best_match_size);
    }

    while
      ((best_match_lower_bound + best_match_size as u8) < build_high) &&
      ((best_match_upper_bound + best_match_size as u8) > other_build_high) &&
      (build[(best_match_lower_bound + best_match_size as u8) as usize] == other_build[(best_match_upper_bound + best_match_size as u8) as usize])
    {
      best_match_size += 1;
    }

    (best_match_lower_bound, best_match_upper_bound, best_match_size as u8)
  }

  // https://github.com/python/cpython/blob/c6b84a727c9299f24edbab4105ce47e9f2bae199/Lib/difflib.py#L421
  fn get_matching_blocks<S: Into<String> + Eq + Hash + Debug>(
    build: &Vec<S>,
    other_build: &Vec<S>,
  ) -> Vec<(u8, u8, u8)> {
    let mut queue: Vec<(u8, u8, u8, u8)> = vec![(0, build.len() as u8, 0, other_build.len() as u8)];
    let mut matching_blocks = vec![];

    while queue.len() != 0 {
      let (
        build_low_index,
        build_high_index,
        other_build_low_index,
        other_build_high_index,
      ) = queue.pop().unwrap();

      let longest_match = Builds::find_longest_match(
        build,
        other_build,
        build_low_index,
        build_high_index,
        other_build_low_index,
        other_build_high_index,
      );
      println!("longest match {:?}", longest_match);
      let (build_match_index, other_build_match_index, match_length) = longest_match;

      if match_length != 0 {
        matching_blocks.push(longest_match);
        if
          build_low_index < build_match_index &&
          other_build_low_index < other_build_match_index
        {
          queue.push((
            build_low_index,
            build_match_index,
            other_build_low_index,
            other_build_match_index,
          ));
        }

        if
          (build_match_index + match_length) < build_high_index &&
          (other_build_match_index + match_length) < other_build_high_index
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
      if
        (previous_build_match_index + previous_match_length) == *build_match_index &&
        (previous_other_build_match_index + previous_match_length) == *other_build_match_index
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
        ) = (
          *build_match_index,
          *other_build_match_index,
          *match_length,
        )
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
    let mut build_path_mappings: HashMap<&String, Vec<&str>> = HashMap::new();
    let mut build_building_count: HashMap<String, u8> = HashMap::new();
    for (path, _, _) in &self.token_paths {
      let mapped_path: Vec<&str> = path.split(
        |c| c == TOKEN_SEPARATOR || c == BUILDING_SEPARATOR_CHAR
      ).collect();

      for building in &mapped_path {
        let building_identifier = format!("{}__{building}", mapped_path.join(","));
        build_building_count
          .entry(building_identifier)
          .and_modify(|count| *count += 1)
          .or_insert(1);
      }

      build_path_mappings.insert(path, mapped_path);
    }

    let mut build_matching_buildings = vec![];
    let mut other_build_matching_buildings = vec![];
    let mut match_missing_building_count: HashMap<&str, u8> = HashMap::new();
    for (path, build) in &build_path_mappings {
      for (other_path, other_build) in &build_path_mappings {
        if path == other_path {
          continue;
        }

        let build_matchup = path
          .split(TOKEN_SEPARATOR).collect::<Vec<&str>>()[0]
          .split(MATCHUP_SEPARATOR).collect::<Vec<&str>>()[1];
        let other_build_matchup = other_path
          .split(TOKEN_SEPARATOR).collect::<Vec<&str>>()[0]
          .split(MATCHUP_SEPARATOR).collect::<Vec<&str>>()[1];

        // only generate comparisons for builds from the same matchup
        if build_matchup != other_build_matchup {
          continue;
        }

        if build.join(BUILDING_SEPARATOR) == other_build.join(BUILDING_SEPARATOR) {
          // information == 0
        }

        // this is all wrong right now
        for matching_block in Builds::get_matching_blocks(build, other_build) {
          for i in matching_block.0..matching_block.0 + matching_block.2 {
            build_matching_buildings.push(build[i as usize]);
          }

          let mut prev_building = build_matching_buildings[0];
          let mut missing_building_count = 0;
          for building in &build_matching_buildings {
            if *building == prev_building {
              missing_building_count += 1;
            } else {
              match_missing_building_count.insert(prev_building, missing_building_count);
              prev_building = building;
              missing_building_count = 1;
            }
          }

          // -----

          for i in matching_block.1..matching_block.1 + matching_block.2 {
            other_build_matching_buildings.push(other_build[i as usize]);
          }
          other_build_matching_buildings.sort();

          let mut other_prev_building = other_build_matching_buildings[0];
          let mut other_missing_building_count = 0;
          for building in &other_build_matching_buildings {
            if *building == other_prev_building {
              other_missing_building_count += 1;
            } else {
              match_missing_building_count.insert(other_prev_building, other_missing_building_count);
              other_prev_building = building;
              other_missing_building_count = 1;
            }
          }
        }

        // calculate and sum td-idf values for each building in both builds being compared which is not a match
        // this calculates the total information difference between builds

      }

      // reset matching buildings for next build comparison
      build_matching_buildings.clear();
      other_build_matching_buildings.clear();
    }
  }

  // pub fn generate_clusters(&mut self) {
  //   while true {

  //   }
  //   cluster_comparisons = {}
  //   for (build_id, other_id), diff in build_comparisons.items():
  //       if build_id in build_clusters and other_id in build_clusters:
  //           cluster_comparisons[(build_id, other_id)] = diff

  //   if not cluster_comparisons:
  //       break

  //   sorted_comparisons = sorted(
  //       cluster_comparisons.items(),
  //       key=lambda build: build[1],
  //   )

  //   completed = False
  //   for min_comparison_builds, min_comparison_diff in sorted_comparisons:
  //       if min_comparison_diff > MAX_COMPARISON_DIFF:
  //           break

  //       # cross check cluster builds
  //       cluster_complete_linkage = True
  //       for build_id in min_comparison_builds:
  //           other_comparison_id = min_comparison_builds[0] if min_comparison_builds[1] == build_id else min_comparison_builds[1]
  //           for other_id in build_clusters[build_id]:
  //               cross_cluster_diff = build_comparisons[tuple(sorted([other_comparison_id, other_id]))]
  //               if cross_cluster_diff > MAX_COMPARISON_DIFF:
  //                   # print(other_comparison_id, other_id, cross_cluster_diff)
  //                   cluster_complete_linkage = False
  //                   break

  //           if not cluster_complete_linkage:
  //               break

  //       if not cluster_complete_linkage:
  //           continue

  //       max_build_count = -1
  //       max_build_id = None
  //       for build_id in min_comparison_builds:
  //           if build_list[build_id][1] > max_build_count:
  //               max_build_count = build_list[build_id][1]
  //               max_build_id = build_id

  //       other_build_id = min_comparison_builds[0] if min_comparison_builds[1] == max_build_id else min_comparison_builds[1]
  //       build_clusters[max_build_id].extend(build_clusters[other_build_id])
  //       build_clusters[max_build_id].append(other_build_id)
  //       del build_clusters[other_build_id]
  //       completed = True
  //       break

  //   if not completed:
  //       break

  // }
}
