use crate::replay::Replay;

use std::fs::{read_dir};
use std::io::Result;
use std::path::Path;

use serde::{Serialize, Deserialize};
use sha256::digest_file;

#[derive(Serialize, Deserialize)]
struct Manifest {
  content_hashes: Vec<String>,
}

pub fn visit_dirs(replays: &mut Vec<Replay>, dir: &Path) -> Result<()> {
  const VALID_TAGS: [&str; 8] = [
    "ASUS ROG",
    "DreamHack Masters",
    "HomeStory Cup",
    "IEM Katowice",
    "TSL",
    "Wardi",
    "Olimoleague",
    "AlphaX",
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
            replays.push(Replay::new(path, content_hash, tags));
          }
        },
        None => continue,
      }
    }
  }
  Ok(())
}
