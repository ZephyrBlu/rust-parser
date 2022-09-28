use std::collections::HashMap;

pub struct BuildTokens {
  pub tokens: HashMap<String, u32>,
  pub probability: HashMap<String, f32>,
  pub information: HashMap<String, f32>,
  pub token_groupings: HashMap<String, Vec<String>>,
  build_token_paths: Vec<(String, f32)>,
  pub token_paths: Vec<(String, f32)>,
  pub skipped_builds: Vec<String>,
}

const MAX_TOKEN_SIZE: usize = 4;

impl BuildTokens {
  pub fn new() -> BuildTokens {
    BuildTokens {
      tokens: HashMap::new(),
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
      for window_size in 1..9 {
        let tokens = &build[i..i + window_size];
        let mut current_token = tokens[0].clone();
        let mut next_token = "NONE";

        if tokens.len() > 1 {
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
    let mut predicted_tokens: HashMap<String, u32> = HashMap::new();
    for (key, count) in &self.tokens {
      // not enough occurrences to generate a meaningful probability
      if *count < 10 {
        continue;
      }

      let values: Vec<&str> = key.split("__").collect();
      let prefix = values[0];
      let token = values[1];

      let split_token: Vec<&str> = token.split(",").collect();
      let mut current_token = String::from(split_token[0]);
      let mut next_token = "NONE";

      if split_token.len() > 1 {
        current_token = split_token[..split_token.len() - 1].join(",");
        next_token = split_token[split_token.len() - 1];
      }

      let matchup_token = format!("{prefix}__{current_token}");

      let identifier_token = format!(
        "{prefix}__{current_token}__{next_token}");

      token_totals.entry(matchup_token).and_modify(|count| *count += 1).or_insert(1);
      predicted_tokens.entry(identifier_token).and_modify(|count| *count += 1).or_insert(1);
    }

    for (key, count) in &predicted_tokens {
      let values: Vec<&str> = key.split("__").collect();
      let prefix = values[0];
      let current_token = values[1];
      // let next_token = values[2];

      let matchup_token = format!("{prefix}__{current_token}");
      let current_token_total = match token_totals.get(&matchup_token) {
        Some(total) => total,
        None => panic!("Couldn't find total for current token: {:?}", current_token),
      };

      let probability = *count as f32 / *current_token_total as f32;
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
    for token_window in 1..(MAX_TOKEN_SIZE + 1) {
      // base case for recursion
      if build_index + token_window > build.len() {
        // ensure we have a path the same size as our original build
        // paths may be shorter if the last token was skipped due to low probability
        if current_path_length == build.len() {
          self.build_token_paths.push((current_path, path_probability));
        }
        break;
      }

      let tokens = &build[build_index..build_index + token_window];

      // assume unigram. e.g. only 1 token is present
      let mut current_token = tokens[0].clone();
      let mut next_token = "NONE";

      // if more than unigram, update values
      if tokens.len() > 1 {
        current_token = tokens[..tokens.len() - 1].join(",");
        next_token = &tokens[tokens.len() - 1];
      }

      let mut next_path_probability = path_probability.clone();
      let identifier_token = format!("{token_prefix}__{current_token}__{next_token}");

      // if we don't have a conditional probability for the tokens there was < 10 occurrences
      if let Some(token_probability) = self.probability.get(&identifier_token) {
        next_path_probability *= token_probability;
      } else {
        continue;
      };

      let mut next_path = current_path.clone();
      if next_path != "" {
        next_path.push('|');
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
}
