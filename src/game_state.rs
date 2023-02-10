use std::collections::HashMap;

use serde::Serialize;

type ObjectCount = HashMap<String, u8>;

#[derive(Serialize)]
pub struct GameState {
  pub gameloop: u16,
  pub buildings: ObjectCount,
  pub units: ObjectCount,
}

impl GameState {
  pub fn new() -> GameState {
    GameState {
      gameloop: 0,
      buildings: HashMap::new(),
      units: HashMap::new(),
    }
  }

  pub fn reset(&mut self) {
    self.gameloop = 0;
    self.buildings.clear();
    self.units.clear();
  }

  pub fn from(gameloop: u16, buildings: ObjectCount, units: ObjectCount) -> GameState {
    GameState {
      gameloop,
      buildings,
      units,
    }
  }
}
