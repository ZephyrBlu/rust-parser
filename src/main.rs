mod decoders;
mod protocol;
mod mpq;
mod replay;

use crate::replay::Replay;

use std::fs::read_dir;
use std::io::Result;

use std::path::Path;
// use bzip2_rs::ParallelDecoderReader;
// use bzip2_rs::RayonThreadPool;

fn visit_dirs(replays: &mut Vec<Replay>, dir: &Path) -> Result<()> {
  if dir.is_dir() {
    for entry in read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      // let filename = entry.file_name();
      if path.is_dir() {
        visit_dirs(replays, &entry.path())?;
      }

      match path.extension() {
        Some(extension) => {
          if extension == "SC2Replay" {
            replays.push(Replay::new(path));
          }
        },
        None => continue,
      }
    }
  }
  Ok(())
}

use std::time::Instant;

// #[global_allocator]
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
  let now = Instant::now();

  let replay_dir = Path::new("/Users/lukeholroyd/Desktop/replays/structured/DreamHack Masters/");
  let mut replays: Vec<Replay> = vec![];
  visit_dirs(&mut replays, replay_dir).unwrap();

  let num_replays = replays.len();
  println!("visited {:?} files in {:.2?}", num_replays, now.elapsed());

  for mut replay in replays {
    replay.parse();
  }

  println!("{:?} replays parsed in {:.2?}, {:?} per replay", num_replays, now.elapsed(), now.elapsed() / num_replays as u32);
}
