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
mod search;
mod cluster;
mod game_state;

use crate::parser::ReplayParser;
use crate::replay::Replay;
use crate::index::Index;
use crate::utils::visit_dirs;
use crate::builds::Builds;

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use csv::Writer;
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

#[derive(Clone, Serialize)]
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
  Builds([Vec<String>; 2]),
  BuildMappings([u16; 2]),
  Winner(u8),
  GameLength(u16),
  Map(String),
  PlayedAt(u64),
  SummaryStats(HashMap<u8, HashMap<&'a str, SummaryStat>>),
  Metadata(String),
  Tinybird(TinybirdGame),
}

type ReplaySummary<'a> = HashMap<&'a str, ReplayEntry<'a>>;

#[derive(Serialize)]
struct SerializedReplays<'a> {
  #[serde(borrow)]
  replays: Vec<ReplaySummary<'a>>,
}

#[derive(Clone, Serialize)]
pub struct TinybirdGame {
  content_hash: String,
  winner_id: u8,
  winner_name: String,
  winner_race: String,
  winner_build: String,
  loser_id: u8,
  loser_name: String,
  loser_race: String,
  loser_build: String,
  matchup: String,
  players: String,
  player_names: String,
  builds: String,
  map: String,
  game_length: u16,
  played_at: u64,
  event: String,
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
  let mut replay_builds: Vec<String> = vec![];
  let mut result = SerializedReplays {
    replays: replay_summaries,
  };

  let mut tinybird_serialized: Vec<TinybirdGame> = vec![];

  let mut race_index = Index::new("race");
  let mut player_index = Index::new("player");
  let mut metadata_index = Index::new("metadata");
  let mut map_index = Index::new("map");
  let mut build_index = Index::new("build");

  let mut replay_id = 0;
  let replay_parser = ReplayParser::new();

  let mut build_tokens = Builds::new();

  for replay in replays {
    let content_hash = replay.content_hash.clone();
    // don't include replays we've seen before
    if seen_replays.contains(&content_hash) {
      continue;
    }

    let mut replay_summary = match replay_parser.parse_replay(
      replay,
      &mut replay_builds,
    ) {
      Ok(summary) => summary,
      Err(e) => {
        // println!("Error parsing replay: {e}");
        continue;
      },
    };
    replay_summary.insert("id", ReplayEntry::Id(replay_id));
    replay_summary.insert("content_hash", ReplayEntry::ContentHash(content_hash.clone()));

    if let ReplayEntry::Map(map) = replay_summary.get("map").unwrap() {
      map_index.add_id(map.to_lowercase().clone(), replay_id);
      map_index.add_hash(map.to_lowercase().clone(), content_hash.clone());
    }

    if let ReplayEntry::Metadata(metadata) = replay_summary.get("metadata").unwrap() {
      for tag in metadata.split(",") {
        metadata_index.add_id(tag.to_lowercase().to_string(), replay_id);
        metadata_index.add_hash(tag.to_lowercase().to_string(), content_hash.clone());
      }
    }

    if let ReplayEntry::Tinybird(tinybird) = replay_summary.get("tinybird").unwrap() {
      if tinybird.winner_build != "" && tinybird.loser_build != "" {
        tinybird_serialized.push(tinybird.clone());
      }
    }

    let mut races = vec![];
    let mut matchup = vec![];
    if let ReplayEntry::Players(players) = replay_summary.get("players").unwrap() {
      for player in players {
        races.push(player.race.clone());
        matchup.push(player.race.clone());

        race_index.add_id(player.race.to_lowercase().clone(), replay_id as u32);
        race_index.add_hash(player.race.to_lowercase().clone(), content_hash.clone());

        player_index.add_id(player.name.to_lowercase().clone(), replay_id as u32);
        player_index.add_hash(player.name.to_lowercase().clone(), content_hash.clone());
      }
    }
    matchup.sort();

    if let ReplayEntry::BuildMappings(builds) = replay_summary.get("build_mappings").unwrap() {
      let matchup_prefix = matchup.join(",");
      for (p_id, player_build_index) in builds.iter().enumerate() {
        let player_build = replay_builds[*player_build_index as usize].split(",").map(|s| s.to_string()).collect();
        let token_prefix = format!("{}-{}", races[p_id], matchup_prefix);

        let mut win = false;
        if let ReplayEntry::Winner(winner_id) = replay_summary.get("winner").unwrap() {
          win = (p_id + 1) == *winner_id as usize;
        }
        build_tokens.generate_tokens(&player_build, win, token_prefix);

        if player_build.len() <= 3 {
          // println!("Build has less than 3 buildings: {:?}", player_build);
          continue;
        }

        // // println!("Full build: {:?}", player_build);
        // for i in 0..(player_build.len() - 2) {
        //   let trigram = &player_build[i..(i + 3)];
        //   // println!("Generated trigram: {:?}", trigram);
        //   build_index.add(trigram.join(","), replay_id as u32);
        // }
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

    // if let ReplayEntry::Builds(builds) = replay_summary.get("builds").unwrap() {
    //   let matchup_prefix = matchup.join(",");
    //   for (p_id, player_build) in builds.iter().enumerate() {
    //     if player_build.len() <= 3 {
    //       // println!("Build has less than 3 buildings: {:?}", player_build);
    //       skipped_builds += 1;
    //       continue;
    //     }

    //     let build_prefix = format!("{}-{}", races[p_id], matchup_prefix);
    //     build_tokens.generate_token_paths(&player_build, build_prefix);
    //   }
    // }
  }

  // // sort by token path probabilities
  // build_tokens.token_paths
  //   .sort_by(|a, b|
  //     a.1
  //       .partial_cmp(&b.1)
  //       .expect("path probabilities should be floats"));

  // println!("generated token paths in {:.2?}", now.elapsed() - token_path_time);
  // println!("skipped builds: {:?}", skipped_builds + build_tokens.skipped_builds.len());
  // println!("total paths: {:?}", build_tokens.token_paths.len());

  // println!("comparing builds");
  // build_tokens.compare_builds();
  // println!("generating build clusters");
  // build_tokens.generate_clusters();

  // let mut build_information = vec![];
  // for (builds, information) in &build_tokens.build_comparison_information {
  //   build_information.push((information, builds));
  // }
  // build_information.sort_by(|a, b|
  //   a.0
  //     .partial_cmp(&b.0)
  //     .expect("path probabilities should be floats"));
  // build_information.reverse();

  // // for (information, builds) in build_information {
  // //   println!("{:?} {:?}", information, builds);
  // // }
  // println!("generated {:?} comparisons", build_tokens.build_comparison_information.len());

  build_tokens.generate_matchup_build_trees();

  // let mut comparison_mappings: HashMap<u32, &String> = HashMap::new();
  // for (index, (comparison_identifier, comparison_diff)) in build_tokens.build_comparison_information.iter().enumerate() {
  //   comparison_mappings.insert(index as u32, comparison_identifier);
  //   result.comparisons.insert(index as u32, comparison_diff);
  // }

  println!("{:?} replays parsed in {:.2?}, {:?} per replay", num_replays, now.elapsed(), now.elapsed() / num_replays as u32);

  // ----------------------------------------------------------------------

  // let indexes = vec![
  //   &race_index,
  //   &player_index,
  //   &map_index,
  //   &metadata_index,
  // ];

  // let mut queries = vec![];
  // for index in &indexes {
  //   for key in index.hash_entries.keys() {
  //     queries.push(key.to_lowercase());
  //   }
  // }

  // let mut search = Search::new();

  // for query in queries {
  //   search.search(query, &indexes);
  // }

  // let mut replay_search_results: HashMap<&String, Vec<&ReplaySummary>> = HashMap::new();
  // for (query_key, references) in &search.results {
  //   for id in references {
  //     let replay = &result.replays[*id as usize];
  //     replay_search_results
  //       .entry(query_key)
  //       .and_modify(|references| references.push(replay))
  //       .or_insert(vec![replay]);
  //   }
  // }

  // let results_output = File::create("../search/data/computed.json").unwrap();
  // serde_json::to_writer(&results_output, &replay_search_results);

  let players_output = File::create("../search/data/players.json").unwrap();
  serde_json::to_writer(&players_output, &build_tokens.players);

  let mut mapped_replays = HashMap::new();
  for replay in &result.replays {
    if let ReplayEntry::ContentHash(value) = replay.get("content_hash").unwrap() {
      mapped_replays.insert(value, replay);
    }
  }
  let replay_output = File::create("../search/data/replays.json").unwrap();
  serde_json::to_writer(&replay_output, &mapped_replays);

  // let build_comparisons_output = File::create("../search/data/comparisons.json").unwrap();
  // serde_json::to_writer(&build_comparisons_output, &build_tokens.build_comparison_information);

  let token_probability_output = File::create("../search/data/probability.json").unwrap();
  serde_json::to_writer(&token_probability_output, &build_tokens.probability);

  let build_output = File::create("../search/data/builds.json").unwrap();
  serde_json::to_writer(&build_output, &build_tokens.builds);

  let cluster_output = File::create("../search/data/clusters.json").unwrap();
  serde_json::to_writer(&cluster_output, &build_tokens.build_clusters);

  let tree_output = File::create("../search/data/build_tree.json").unwrap();
  serde_json::to_writer(&tree_output, &build_tokens.build_tree);

  let raw_tree_output = File::create("../search/data/raw_build_tree.json").unwrap();
  serde_json::to_writer(&raw_tree_output, &build_tokens.raw_build_tree);

  let player_trees_output = File::create("../search/data/player_trees.json").unwrap();
  serde_json::to_writer(&player_trees_output, &build_tokens.player_trees);

  let build_token_output = File::create("../search/data/tokens.json").unwrap();
  serde_json::to_writer(&build_token_output, &build_tokens.build_token_path_mappings);

  File::create("tinybird_sc2.csv").unwrap();
  let mut wtr = Writer::from_path("tinybird_sc2.csv").unwrap();
  for record in tinybird_serialized {
    wtr.serialize(record).unwrap();
  }
  wtr.flush().unwrap();

  // let mut filtered_build_index = Index::new();
  // for (trigram, references) in build_index.entries {
  //   if references.len() >= 10 {
  //     filtered_build_index.entries.insert(trigram, references);
  //   }
  // }

  // let indexes = HashMap::from([
  //   ("race", race_index.hash_entries),
  //   ("player", player_index.hash_entries),
  //   ("metadata", metadata_index.hash_entries),
  //   ("map", map_index.hash_entries),
  //   // ("build", filtered_build_index),
  // ]);

  // let index_output = File::create("../search/data/indexes.json").unwrap();
  // serde_json::to_writer(&index_output, &indexes);

  // let mut build_mappings: Vec<Vec<String>> = vec![];
  // for build in replay_builds {
  //   let split_build = build.split(",").map(|s| s.to_string()).collect();
  //   build_mappings.push(split_build);
  // }

  // let builds_output = File::create("../search/data/builds.json").unwrap();
  // serde_json::to_writer(&builds_output, &build_mappings);

  // let mut compressed_clusters: HashMap<&str, (Vec<(&str, u16)>, u16)> = HashMap::new();

  // let mappings = HashMap::from([
  //   ("clusters", ),
  // ]);

  // let mappings_output = File::create("../sc2.gg/src/assets/mappings.json").unwrap();
  // serde_json::to_writer(&mappings_output, &mappings);

  println!("replays serialized in {:?}", now.elapsed());
}
