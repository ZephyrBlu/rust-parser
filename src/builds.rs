use std::collections::HashMap;

pub struct BuildTokens {
  pub tokens: HashMap<String, u32>,
  pub probability: HashMap<String, f32>,
  pub information: HashMap<String, f32>,
}

impl BuildTokens {
  pub fn new() -> BuildTokens {
    BuildTokens {
      tokens: HashMap::new(),
      probability: HashMap::new(),
      information: HashMap::new(),
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

      println!("Current token token: {:?}", current_token);
      println!("Next token: {:?}", next_token);
      println!("Distribution token: {:?}", distribution_token);

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
      let next_token = values[2];

      let current_token_total = match token_totals.get(current_token) {
        Some(total) => total,
        None => panic!("Couldn't find total for current token: {:?}", current_token),
      };

      println!("Token key: {:?} {:?}", key, count);
      println!("Current token: {:?} {:?}", current_token, current_token_total);
      println!("Next token: {:?} {:?}\n", next_token, count);

      if *current_token_total < 10 {
        continue;
      }

      let probability = *count as f32 / *current_token_total as f32;
      self.probability.insert(key.to_string(), probability);
    }
  }
}
