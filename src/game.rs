pub struct Game {
  pub workers_active: [u8; 2],
  pub minerals_collected: [u16; 2],
  pub minerals_lost: [u16; 2],
  pub gas_collected: [u16; 2],
  pub gas_lost: [u16; 2],
  pub collection_rate: Vec<Vec<(u16, u16)>>,
  pub unspent_resources: Vec<Vec<(u16, u16)>>,
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

    Game {
      workers_active,
      minerals_collected,
      minerals_lost,
      gas_collected,
      gas_lost,
      collection_rate,
      unspent_resources,
    }
  }
}
