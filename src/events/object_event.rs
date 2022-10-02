use crate::game::Game;
use crate::replay::Event;
use crate::decoders::DecoderResult;

// doesn't include supply structures, gas collectors and support structures
const BUILDINGS: [&str; 42] = [
  // Protoss
  "Nexus",
  "Gateway",
  "Forge",
  "CyberneticsCore",
  "PhotonCannon", // we'll see about this one
  "RoboticsFacility",
  "Stargate",
  "TwilightCouncil",
  "RoboticsBay",
  "FleetBeacon",
  "TemplarArchives",
  "DarkShrine",

  // Terran
  "CommandCenter",
  "OrbitalCommand",
  "PlanetaryFortress",
  "Barracks",
  "EngineeringBay",
  "GhostAcademy",
  "Factory",
  "Starport",
  "Armory",
  "FusionCore",
  "BarracksTechLab",
  "FactoryTechLab",
  "StarportTechLab",
  "BarracksReactor",
  "FactoryReactor",
  "StarportReactor",

  // Zerg
  "Hatchery",
  "SpawningPool",
  "EvolutionChamber",
  "RoachWarren",
  "BanelingNest",
  "Lair",
  "HydraliskDen",
  "LurkerDenMP",
  "Spire",
  "GreaterSpire",
  "NydusNetwork",
  "InfestationPit",
  "Hive",
  "UltraliskCavern",
];

pub struct ObjectEvent;

impl ObjectEvent {
  pub fn new(game: &mut Game, event: &Event) -> Result<(), &'static str> {
    let mut player_id: u8 = 0;
    let mut building_name = String::new();
    let mut tag_index = 0;
    let mut tag_recycle = 0;
    let mut current_gameloop = 0;
    // println!("event entry values {:?}", event.entries);
    for (field, value) in &event.entries {
      match field.as_str() {
        "m_controlPlayerId" => player_id = if let DecoderResult::Value(v) = value {
          *v as u8
        } else {
          return Err("Player ID is not a value");
        },
        "m_unitTypeName" => if let DecoderResult::Blob(unit_name) = value {
          if BUILDINGS.contains(&unit_name.as_str()) {
            building_name = unit_name.to_string();
          }
        },
        "m_unitTagIndex" => if let DecoderResult::Value(index) = value {
          tag_index = *index as u32;
        },
        "m_unitTagRecycle" => if let DecoderResult::Value(recycle) = value{
          tag_recycle = *recycle as u32;
        },
        "_gameloop" => if let DecoderResult::Value(gameloop) = value {
          // ~7min, 22.4 gameloops per sec
          if *gameloop > 9408 {
            return Err("Gameloop is past 7min");
          }
          current_gameloop = *gameloop;
        },
        _other => continue,
      }
    }

    if building_name == "" {
      return Err("Building not found");
    }

    let tag = (tag_index << 18) + tag_recycle;
    let player_index = match game.buildings.get(&tag) {
      Some(building_player_id) => {
        building_player_id - 1
      },
      None => {
        game.buildings.insert(tag, player_id);
        player_id - 1
      },
    };

    if player_index > 1 {
      return Err("More than 2 players in replay");
    }

    if game.builds[player_index as usize].len() < 10 && current_gameloop > 0 {
      game.builds[player_index as usize].push(building_name);
    }

    Ok(())
  }
}
