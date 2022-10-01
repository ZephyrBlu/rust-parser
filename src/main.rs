mod decoders;
mod protocol;
mod mpq;
mod replay;
mod index;
mod events;
mod utils;
mod game;
mod parser;
mod builds;

use crate::parser::ReplayParser;
use crate::replay::Replay;
use crate::index::Index;
use crate::utils::visit_dirs;
use crate::builds::Builds;

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
// use bzip2_rs::ParallelDecoderReader;
// use bzip2_rs::RayonThreadPool;

use std::time::Instant;

// #[global_allocator]
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Serialize)]
#[serde(untagged)]
pub enum SummaryStat {
  ResourceValues((u16, u16)),
  Value(u16),
}

#[derive(Serialize)]
pub struct Player {
  id: u8,
  name: String,
  race: String,
  // build: Vec<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ReplayEntry<'a> {
  Id(u32),
  ContentHash(String),
  Players(Vec<Player>),
  Builds(Vec<Vec<String>>),
  Winner(u8),
  GameLength(u16),
  Map(String),
  PlayedAt(u64),
  SummaryStats(HashMap<u8, HashMap<&'a str, SummaryStat>>),
  Metadata(String),
}

type ReplaySummary<'a> = HashMap<&'a str, ReplayEntry<'a>>;

#[derive(Serialize)]
struct SerializedReplays<'a> {
  #[serde(borrow)]
  replays: Vec<ReplaySummary<'a>>,
}

fn main() {
  let now = Instant::now();

  // let replay_dir = Path::new("/Users/lukeholroyd/Desktop/replays/structured/IEM Katowice/2022/1 - Round of 36 - Play-ins/01 - UB Ro16 - ByuN vs Percival/");
  let replay_dir = Path::new("/Users/lukeholroyd/Desktop/replays/structured/");
  let mut replays: Vec<Replay> = vec![];
  let mut seen_replays: HashSet<String> = HashSet::new();
  visit_dirs(&mut replays, replay_dir).unwrap();

  let num_replays = replays.len();
  println!("visited {:?} files in {:.2?}", num_replays, now.elapsed());

  let replay_summaries: Vec<ReplaySummary> = vec![];
  let mut result = SerializedReplays {
    replays: replay_summaries,
  };

  let mut race_index = Index::new();
  let mut player_index = Index::new();
  let mut metadata_index = Index::new();
  let mut map_index = Index::new();
  let mut build_index = Index::new();

  let mut replay_id = 0;
  let replay_parser = ReplayParser::new();

  let mut build_tokens = Builds::new();

  for replay in replays {
    let content_hash = replay.content_hash.clone();
    // don't include replays we've seen before
    if seen_replays.contains(&content_hash) {
      continue;
    }

    let mut replay_summary = match replay_parser.parse_replay(replay.parsed) {
      Ok(summary) => summary,
      Err(e) => {
        // println!("Error parsing replay: {e}");
        continue;
      },
    };
    replay_summary.insert("id", ReplayEntry::Id(replay_id));
    replay_summary.insert("content_hash", ReplayEntry::ContentHash(content_hash.clone()));

    if let ReplayEntry::Map(map) = replay_summary.get("map").unwrap() {
      map_index.add(map.clone(), replay_id);
    }

    if let ReplayEntry::Metadata(metadata) = replay_summary.get("metadata").unwrap() {
      for tag in metadata.split(", ") {
        metadata_index.add(tag.to_string(), replay_id);
      }
    }

    let mut races = vec![];
    let mut matchup = vec![];
    if let ReplayEntry::Players(players) = replay_summary.get("players").unwrap() {
      for player in players {
        races.push(player.race.clone());
        matchup.push(player.race.clone());

        race_index.add(player.race.clone(), replay_id as u32);
        player_index.add(player.name.clone(), replay_id as u32);
      }
    }
    matchup.sort();

    if let ReplayEntry::Builds(builds) = replay_summary.get("builds").unwrap() {
      let matchup_prefix = matchup.join(",");
      for (p_id, player_build) in builds.iter().enumerate() {
        let token_prefix = format!("{}-{}", races[p_id], matchup_prefix);
        build_tokens.generate_tokens(player_build, token_prefix);

        if player_build.len() <= 3 {
          // println!("Build has less than 3 buildings: {:?}", player_build);
          continue;
        }

        // println!("Full build: {:?}", player_build);
        for i in 0..(player_build.len() - 2) {
          let trigram = &player_build[i..(i + 3)];
          // println!("Generated trigram: {:?}", trigram);
          build_index.add(trigram.join(","), replay_id as u32);
        }
      }
    }

    result.replays.push(replay_summary);
    seen_replays.insert(content_hash);
    replay_id += 1;
  }

  let distribution_time = now.elapsed();
  build_tokens.generate_token_distributions();
  println!("generated token distributions in {:.2?}", now.elapsed() - distribution_time);

  let token_path_time = now.elapsed();
  let mut skipped_builds = 0;
  for replay_summary in &result.replays {
    let mut races = vec![];
    let mut matchup = vec![];
    if let ReplayEntry::Players(players) = replay_summary.get("players").unwrap() {
      for player in players {
        races.push(player.race.clone());
        matchup.push(player.race.clone());
      }
    }
    matchup.sort();

    if let ReplayEntry::Builds(builds) = replay_summary.get("builds").unwrap() {
      let matchup_prefix = matchup.join(",");
      for (p_id, player_build) in builds.iter().enumerate() {
        if player_build.len() <= 3 {
          // println!("Build has less than 3 buildings: {:?}", player_build);
          skipped_builds += 1;
          continue;
        }

        let build_prefix = format!("{}-{}", races[p_id], matchup_prefix);
        build_tokens.generate_token_paths(player_build, build_prefix);
      }
    }
  }

  // sort by token path probabilities
  build_tokens.token_paths
    .sort_by(|a, b|
      a.1
        .partial_cmp(&b.1)
        .expect("path probabilities should be floats"));

  let mut set = HashSet::new();
  for (path, _, _) in &build_tokens.token_paths {
    set.insert(path);
  }
  println!("generated token paths in {:.2?}", now.elapsed() - token_path_time);
  println!("skipped builds: {:?}", skipped_builds + build_tokens.skipped_builds.len());
  println!("total paths: {:?}, unique paths: {:?}", build_tokens.token_paths.len(), set.len());


  build_tokens.compare_builds();
  let mut build_information = vec![];
  for (builds, information) in &build_tokens.build_comparison_information {
    build_information.push((information, builds));
  }
  build_information.sort_by(|a, b|
    a.0
      .partial_cmp(&b.0)
      .expect("path probabilities should be floats"));
  build_information.reverse();

  // for (information, builds) in build_information {
  //   println!("{:?} {:?}", information, builds);
  // }
  println!("generated {:?} comparisons", build_tokens.build_comparison_information.len());

  println!("{:?} replays parsed in {:.2?}, {:?} per replay", num_replays, now.elapsed(), now.elapsed() / num_replays as u32);

  let replay_output = File::create("../sc2.gg/src/assets/replays.json").unwrap();
  serde_json::to_writer(&replay_output, &result);

  let mut filtered_build_index = Index::new();
  for (trigram, references) in build_index.entries {
    if references.len() >= 10 {
      filtered_build_index.entries.insert(trigram, references);
    }
  }

  let indexes = HashMap::from([
    ("race", race_index),
    ("player", player_index),
    ("metadata", metadata_index),
    ("map", map_index),
    ("build", filtered_build_index),
  ]);

  let index_output = File::create("../sc2.gg/src/assets/indexes.json").unwrap();
  serde_json::to_writer(&index_output, &indexes);

  println!("replays serialized in {:?}", now.elapsed());
}
