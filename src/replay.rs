use crate::decoders::{DecoderResult, EventEntry};
use crate::mpq::MPQArchive;
use crate::protocol::Protocol;

use serde::Deserialize;

use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug)]
pub struct Event<'a> {
  pub entries: Vec<(&'a str, DecoderResult<'a>)>,
}

impl<'a> Event<'a> {
  pub fn new(entries: Vec<(&'a str, DecoderResult<'a>)>) -> Event<'a> {
    Event {
      entries
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct Player<'a> {
  pub PlayerID: u8,
  pub APM: f32,
  pub Result: &'a str,
  pub SelectedRace: &'a str,
  pub AssignedRace: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct Metadata<'a> {
  pub Title: &'a str,
  pub GameVersion: &'a str,
  pub DataBuild: &'a str,
  pub DataVersion: &'a str,
  pub BaseBuild: &'a str,
  pub Duration: u16,
  pub IsNotAvailable: bool,
  pub Players: Vec<Player<'a>>,
}

#[derive(Debug)]
pub struct Parsed<'a> {
  pub player_info: Vec<EventEntry<'a>>,
  pub tracker_events: Vec<Event<'a>>,
  pub metadata: String,
}

pub struct Replay<'a> {
  pub file_path: String,
  pub archive: MPQArchive,
  pub protocol: Protocol,
  pub parsed: Option<Parsed<'a>>,
}

impl<'a> Replay<'a> {
  pub fn new(file_path: PathBuf) -> Replay<'a> {
    let path_str = file_path.to_str().unwrap();
    Replay {
      file_path: path_str.to_string(),
      archive: MPQArchive::new(path_str),
      protocol: Protocol::new(),
      parsed: None,
    }
  }

  pub fn parse (&'a mut self) -> &Parsed<'a> {
    println!("parsing replay {:?}", self.file_path);

    let now = Instant::now();

    let header_content = &self.archive
      .header
      .user_data_header
      .as_ref()
      .expect("No user data header")
      .content;
    // println!("read header {:.2?}", now.elapsed());

    let contents = self.archive.read_file("replay.tracker.events").unwrap();
    // println!("read tracker events {:.2?}", now.elapsed());

    let game_info = self.archive.read_file("replay.game.events").unwrap();
    // println!("read game events {:.2?}", now.elapsed());

    let init_data = self.archive.read_file("replay.initData").unwrap();
    // println!("read details {:.2?}", now.elapsed());

    let raw_metadata = self.archive.read_file("replay.gamemetadata.json").unwrap();
    let metadata = String::from_utf8(raw_metadata.clone()).unwrap();
    // println!("read metadata {:.2?}", now.elapsed());

    let details = self.archive.read_file("replay.details").unwrap();
    let player_info = self.protocol.decode_replay_details(details);

    let tracker_events = self.protocol.decode_replay_tracker_events(contents);
    // println!("decoded replay tracker events {:.2?}", now.elapsed());

    let game_events = self.protocol.decode_replay_game_events(game_info);
    // println!("decoding replay game events {:.2?}", now.elapsed());

    self.parsed = Some(Parsed {
      player_info,
      tracker_events,
      metadata,
    });

    println!("parsed in {:.2?}", now.elapsed());

    self.parsed.as_ref().unwrap()
  }    

  // // function that doesn't parse replay events for speed
  // // can return high level information about game like
  // // date, matchup, MMR, etc to decide whether to skip parsing
  // pub fn peek() {

  // }
}
