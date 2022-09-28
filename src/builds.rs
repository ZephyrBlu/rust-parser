use std::collections::HashMap;

pub struct Builds {
  pub tokens: HashMap<String, u32>,
  cached_token_probability: HashMap<String, f32>,
  pub probability: HashMap<String, f32>,
  pub information: HashMap<String, f32>,
  pub token_groupings: HashMap<String, Vec<String>>,
  build_token_paths: Vec<(String, f32, u8)>,
  pub token_paths: Vec<(String, f32, u8)>,
  pub skipped_builds: Vec<String>,
  // cluster_comparisons: HashMap<>,
}

const MAX_TOKEN_SIZE: usize = 4;
const TOKEN_SEPARATOR: char = '|';

impl Builds {
  pub fn new() -> Builds {
    Builds {
      tokens: HashMap::new(),
      cached_token_probability: HashMap::new(),
      probability: HashMap::new(),
      information: HashMap::new(),
      token_groupings: HashMap::new(),
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
        let mut next_token = "NONE";

        if tokens.len() > 1 && tokens.len() != build.len() {
          current_token = tokens[..tokens.len() - 1].join(",");
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

      let current_token_identifier = if next_token == "NONE" {
        format!("{prefix}__NONE")
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

      let current_token_identifier = if next_token == "NONE" {
        format!("{prefix}__NONE")
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
      self.skipped_builds.push(build.join(","));
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

      // assume unigram. e.g. only 1 token is present
      let mut current_token = tokens[0].clone();
      let mut next_token = "NONE";

      // if more than unigram, update values
      if tokens.len() > 1 && tokens.len() != build.len() {
        current_token = tokens[..tokens.len() - 1].join(",");
        next_token = &tokens[tokens.len() - 1];
      }

      let mut next_path_probability = path_probability.clone();
      let identifier_token = format!("{token_prefix}__{current_token}__{next_token}");

      // if we don't have a conditional probability for the tokens there was < 10 occurrences
      if !self.probability.contains_key(&identifier_token) {
        continue;
      }

      if let Some(token_sequence_probability) = self.cached_token_probability.get(&identifier_token) {
        next_path_probability *= token_sequence_probability;
      } else {
        let mut token_sequence_probability = path_probability.clone();
        let mut token_fragment_window = tokens.len();

        for i in 0..tokens.len() {
          token_fragment_window  -= i;

          // assume unigram. e.g. only 1 token is present
          let mut current_token_fragment = tokens[0].clone();
          let mut next_token_fragment = "NONE";

          // if more than unigram, update values
          if token_fragment_window > 1 && token_fragment_window != build.len() {
            current_token_fragment = tokens[..token_fragment_window - 1].join(",");
            next_token_fragment = &tokens[token_fragment_window - 1];
          }

          let identifier_token_fragment = format!("{token_prefix}__{current_token_fragment}__{next_token_fragment}");

          if let Some(token_fragment_sequence_probability) = self.cached_token_probability.get(&identifier_token_fragment) {
            token_sequence_probability *= token_fragment_sequence_probability;
            break;
          }

          match self.probability.get(&identifier_token_fragment) {
            Some(token_fragment_probability) => token_sequence_probability *= token_fragment_probability,
            None => panic!("Couldn't find fragment probability on iteration {:?} {:?} {:?}", i, current_token, identifier_token_fragment),
          }
        }

        self.cached_token_probability.insert(identifier_token, token_sequence_probability);
        next_path_probability *= token_sequence_probability;
      }

      let mut next_path = current_path.clone();
      if next_path != "" {
        next_path.push(TOKEN_SEPARATOR);
      }
      next_path.push_str(tokens.join(",").as_str());
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

  fn compare_builds(&mut self {

  })

  pub fn generate_clusters(&mut self) {
    while true {

    }
    cluster_comparisons = {}
    for (build_id, other_id), diff in build_comparisons.items():
        if build_id in build_clusters and other_id in build_clusters:
            cluster_comparisons[(build_id, other_id)] = diff

    if not cluster_comparisons:
        break

    sorted_comparisons = sorted(
        cluster_comparisons.items(),
        key=lambda build: build[1],
    )

    completed = False
    for min_comparison_builds, min_comparison_diff in sorted_comparisons:
        if min_comparison_diff > MAX_COMPARISON_DIFF:
            break

        # cross check cluster builds
        cluster_complete_linkage = True
        for build_id in min_comparison_builds:
            other_comparison_id = min_comparison_builds[0] if min_comparison_builds[1] == build_id else min_comparison_builds[1]
            for other_id in build_clusters[build_id]:
                cross_cluster_diff = build_comparisons[tuple(sorted([other_comparison_id, other_id]))]
                if cross_cluster_diff > MAX_COMPARISON_DIFF:
                    # print(other_comparison_id, other_id, cross_cluster_diff)
                    cluster_complete_linkage = False
                    break

            if not cluster_complete_linkage:
                break

        if not cluster_complete_linkage:
            continue

        max_build_count = -1
        max_build_id = None
        for build_id in min_comparison_builds:
            if build_list[build_id][1] > max_build_count:
                max_build_count = build_list[build_id][1]
                max_build_id = build_id

        other_build_id = min_comparison_builds[0] if min_comparison_builds[1] == max_build_id else min_comparison_builds[1]
        build_clusters[max_build_id].extend(build_clusters[other_build_id])
        build_clusters[max_build_id].append(other_build_id)
        del build_clusters[other_build_id]
        completed = True
        break

    if not completed:
        break

  }
}
