use std::collections::HashMap;

pub struct BuildTokens {
  pub tokens: HashMap<String, u32>,
  pub probability: HashMap<String, f32>,
  pub information: HashMap<String, f32>,
  pub token_groupings: HashMap<String, Vec<String>>,
  pub token_paths: Vec<String>,
  pub cached_token_paths: HashMap<String, Vec<String>>,
  cached_token_probability: HashMap<String, Vec<String>>,
}

impl BuildTokens {
  pub fn new() -> BuildTokens {
    BuildTokens {
      tokens: HashMap::new(),
      probability: HashMap::new(),
      information: HashMap::new(),
      token_groupings: HashMap::new(),
      token_paths: vec![],
      cached_token_paths: HashMap::new(),
      cached_token_probability: HashMap::new(),
    }
  }

  pub fn generate_tokens(&mut self, build: &Vec<String>, build_prefix: String) {
    for i in 0..build.len() {
      for window_size in 1..9 {
        let token = format!("{}{}", build_prefix, build[i..i + window_size].join(","));
        // println!("Token with prefix: {:?}", token);
        match self.tokens.get(&token) {
          Some(v) => self.tokens.insert(token, v + 1),
          None => self.tokens.insert(token, 1),
        };

        if i + window_size >= build.len() {
          break;
        }
      }
    }
  }

  pub fn generate_token_distributions(&mut self) {
    let mut token_totals: HashMap<String, u32> = HashMap::new();
    let mut predicted_tokens: HashMap<String, u32> = HashMap::new();
    for (key, _) in &self.tokens {
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

      let distribution_token = format!(
        "{}__{}__{}",
        prefix,
        current_token,
        next_token,
      );

      // println!("Current token token: {:?}", current_token);
      // println!("Next token: {:?}", next_token);
      // println!("Distribution token: {:?}", distribution_token);

      match token_totals.get(&current_token) {
        Some(v) => token_totals.insert(current_token, v + 1),
        None => token_totals.insert(current_token, 1),
      };

      match predicted_tokens.get(&distribution_token) {
        Some(v) => predicted_tokens.insert(distribution_token, v + 1),
        None => predicted_tokens.insert(distribution_token, 1),
      };
    }

    for (key, count) in &predicted_tokens {
      let values: Vec<&str> = key.split("__").collect();
      let current_token = values[1];
      // let next_token = values[2];

      let current_token_total = match token_totals.get(current_token) {
        Some(total) => total,
        None => panic!("Couldn't find total for current token: {:?}", current_token),
      };

      // println!("Token key: {:?} {:?}", key, count);
      // println!("Current token: {:?} {:?}", current_token, current_token_total);
      // println!("Next token: {:?} {:?}\n", next_token, count);

      if *current_token_total < 10 {
        continue;
      }

      let probability = *count as f32 / *current_token_total as f32;
      self.probability.insert(key.to_string(), probability);
    }
  }

  pub fn generate_token_paths(&mut self, build: &Vec<String>, build_prefix: String) {
    self.generate_next_path(
      String::new(),
      build,
      build_prefix.as_str(),
      0,
    );
    // println!("Generated token paths: {:?} {:?}", self.token_paths, self.token_paths.len());
  }

  fn generate_next_path<'a>(
    &mut self,
    current_path: String,
    build: &'a Vec<String>,
    build_prefix: &str,
    build_index: usize,
  ) -> String {
    let max_token_size = 4;
    for token_window in 1..(max_token_size + 1) {
      let tokens = &build[build_index..build_index + token_window];
      let mut current_token = tokens[0].clone();
      let mut next_token = "NONE";

      // println!("Current tokens {:?}", tokens);

      if tokens.len() > 1 {
        current_token = tokens[..tokens.len() - 1].join(",");
        next_token = &tokens[tokens.len() - 1];
      }

      // for tokens, check if fragments are cached
      // if cached, use paths as a base for future function calls


      // // println!("tokens {:?}", tokens);
      // for fragment_index in 0..tokens.len() {
      //   let fragment_window = tokens.len() - fragment_index;
      //   let token_fragment = &tokens[..fragment_window];
      //   // println!("fragment {:?} {:?} {:?}", fragment_index, fragment_window, token_fragment);
      // }

      // let distribution_token = format!(
      //   "{}__{}__{}",
      //    &build_prefix,
      //    current_token,
      //    next_token,
      // );

      // match self.cached_token_paths.get(&distribution_token) {
      //   Some(paths) => ,
      //   None => ,
      // }

      // println!("Distribution token: {:?}", distribution_token);

      // let token_probability = match self.probability.get(&distribution_token) {
      //   Some(probability) => probability,
      //   None => continue,
      // };

      if build_index + token_window >= build.len() {
        // self.token_paths.push(current_path);

        // let serialized_build = build.join(",");
        // match self.cached_token_paths.get_mut(&serialized_build) {
        //   Some(paths) => paths.push(current_path.clone()),
        //   None => {
        //     self.cached_token_paths.insert(serialized_build, vec![current_path.clone()]);
        //   },
        // };

        break;
      }

      let mut next_path = current_path.clone();
      if next_path != "" {
        next_path.push('|');
      }
      next_path.push_str(tokens.join(",").as_str());

      let subpath = self.generate_next_path(
        next_path,
        build,
        build_prefix,
        build_index + token_window,
      );
      // println!("returned subpath {:?} {:?}", build, subpath);
      self.token_paths.push(subpath);
    }

    current_path
  }
}
