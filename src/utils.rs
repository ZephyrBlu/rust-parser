use crate::replay::Replay;

use std::fs::{read_dir, copy};
use std::io::Result;
use std::path::Path;
use std::collections::HashMap;

use sha256::digest_file;

pub fn visit_dirs(replays: &mut Vec<Replay>, dir: &Path) -> Result<()> {
  const TOURNAMENTS: [&str; 6] = [
    "ASUS ROG",
    "DreamHack Masters",
    "HomeStory Cup",
    "IEM Katowice",
    "StayAtHome Story Cup",
    "TSL",
  ];
  const VALID_TAGS: [&str; 10] = [
    "FINAL",
    "SEMIFINAL",
    "QUARTERFINAL",
    "PLAYOFF",
    "GROUP",
    "RO32",
    "RO16",
    "RO8",
    "RO4",
    "RO2",
  ];
  let TAG_MAPPINGS: HashMap<&str, &str> = HashMap::from([
    ("RO2", "Final"),
    ("RO4", "Semifinal"),
    ("RO8", "Quarterfinal"),
    ("FINAL", "Final"),
    ("SEMIFINAL", "Semifinal"),
    ("QUARTERFINAL", "Quarterfinal"),
    ("PLAYOFF", "Playoff"),
    ("GROUP", "Group"),
  ]);

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

            for tag in TOURNAMENTS {
              if current_path.contains(tag) {
                tags.push(tag);
              }
            }

            for tag in VALID_TAGS {
              if current_path.to_uppercase().contains(tag) {
                let mut mapped_tag = tag;
                if TAG_MAPPINGS.contains_key(mapped_tag) {
                  mapped_tag = match TAG_MAPPINGS.get(tag) {
                    Some(value) => value,
                    None => tag,
                  }
                }
                tags.push(mapped_tag);
              }
            }

            let content_hash = digest_file(&path).expect("Replay file should be hashed");
            let bucket_path = format!("/Users/lukeholroyd/Desktop/replays/bucket/{content_hash}.SC2Replay");
            println!("copying replay file to new bucket path: {:?}", bucket_path);
            copy(
              &path,
              bucket_path,
            ).expect("Replay file is copied from existing file structure into bucket structure");
            replays.push(Replay::new(path, content_hash, tags));
          }
        },
        None => continue,
      }
    }
  }
  Ok(())
}
