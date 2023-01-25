use crate::replay::Replay;
use crate::decoders::DecoderResult;

use std::fs::{copy, read_dir};
use std::io::Result;
use std::path::Path;

use serde::{Serialize, Deserialize};
use sha256::digest_file;

#[derive(Serialize, Deserialize)]
struct Manifest {
  content_hashes: Vec<String>,
}

pub fn visit_dirs(replays: &mut Vec<Replay>, dir: &Path) -> Result<()> {
  const VALID_TAGS: [&str; 10] = [
    "ASUS ROG",
    "DreamHack Masters",
    "HomeStory Cup",
    "IEM Katowice",
    "TSL",
    "Wardi",
    "OlimoLeague",
    "AlphaX",
    "WESG",
    "WCS",
  ];

  if dir.is_dir() {
    for entry in read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      // let filename = entry.file_name();
      if path.is_dir() && !path.to_str().unwrap().contains("PiG") {
        visit_dirs(replays, &entry.path())?;
      }

      match path.extension() {
        Some(extension) => {
          if extension == "SC2Replay" {
            let current_path = path.to_str().unwrap();
            let mut tags = vec![];

            for tag in VALID_TAGS {
              if current_path.contains(tag) {
                tags.push(tag);
              }
            }

            let content_hash = digest_file(&path).expect("Replay file should be hashed");
            // let bucket_path = format!("/Users/lukeholroyd/Desktop/replays/bucket/{content_hash}.SC2Replay");
            // println!("copying replay file to new bucket path: {:?}", bucket_path);
            // copy(
            //   &path,
            //   bucket_path,
            // ).expect("Replay file is copied from existing file structure into bucket structure");
            let replay = Replay::new(path, content_hash, tags);

            let raw_played_at = &replay.parsed.player_info
              .iter()
              .find(|(field, _)| *field == "m_timeUTC")
              .unwrap().1;
            let mut played_at = 0;
            if let DecoderResult::Value(value) = raw_played_at {
              // TODO: this truncation is not working properly
              played_at = value.clone() as u64;
            }
            // game records time in window epoch for some reason
            // https://en.wikipedia.org/wiki/Epoch_(computing)
            played_at = (played_at / 10000000) - 11644473600;

            // between 1st Jan 2022 and 1st Jan 2023
            if played_at >= 1640995200 && played_at < 1672531200 {
              replays.push(replay);
            }
          }
        },
        None => continue,
      }
    }
  }
  Ok(())
}
