use std::collections::HashMap;

#[derive(Debug)]
pub struct GameObject {
  pub object_name: String,
  pub object_type: String,
  pub tag_index: u32,
  pub tag_recycle: u32,
}

pub struct Game {
  pub workers_active: [u8; 2],
  pub minerals_collected: [u16; 2],
  pub minerals_lost: [u16; 2],
  pub gas_collected: [u16; 2],
  pub gas_lost: [u16; 2],
  pub collection_rate: Vec<Vec<(u16, u16)>>,
  pub unspent_resources: Vec<Vec<(u16, u16)>>,
  pub builds: Vec<Vec<(String, u16)>>,
  pub buildings: HashMap<u32, u8>,
  pub units: Vec<Vec<(String, u16)>>,
  pub objects: HashMap<u32, GameObject>,
}

impl Game {
  pub fn new() -> Game {
    let workers_active: [u8; 2] = [0, 0];
    let minerals_collected: [u16; 2] = [0, 0];
    let minerals_lost: [u16; 2] = [0, 0];
    let gas_collected: [u16; 2] = [0, 0];
    let gas_lost: [u16; 2] = [0, 0];
    let collection_rate: Vec<Vec<(u16, u16)>> = vec![vec![], vec![]];
    let unspent_resources: Vec<Vec<(u16, u16)>> = vec![vec![], vec![]];
    let builds: Vec<Vec<(String, u16)>> = vec![vec![], vec![]];
    let buildings: HashMap<u32, u8> = HashMap::new();
    let units: Vec<Vec<(String, u16)>> = vec![vec![], vec![]];
    let objects: HashMap<u32, GameObject> = HashMap::new();

    Game {
      workers_active,
      minerals_collected,
      minerals_lost,
      gas_collected,
      gas_lost,
      collection_rate,
      unspent_resources,
      builds,
      buildings,
      units,
      objects,
    }
  }

  pub fn reset(&mut self) {
    self.workers_active = [0, 0];
    self.minerals_collected = [0, 0];
    self.minerals_lost = [0, 0];
    self.gas_collected = [0, 0];
    self.gas_lost = [0, 0];

    for vec in self.collection_rate.iter_mut() {
      vec.clear();
    }

    for vec in self.unspent_resources.iter_mut() {
      vec.clear();
    }

    for vec in self.builds.iter_mut() {
      vec.clear();
    }

    self.buildings.clear();

    for vec in self.units.iter_mut() {
      vec.clear();
    }

    self.objects.clear();
  }
}
