use crate::game::Game;
use crate::replay::Event;
use crate::decoders::DecoderResult;
use crate::game_state::GameState;

const UNITS: [&str; 54] = [
  // Protoss
  "Probe",
  "Zealot",
  "Stalker",
  "Sentry",
  "Adept",
  "HighTemplar",
  "DarkTemplar",
  "Archon",
  "Observer",
  "WarpPrism",
  "Immortal",
  "Colossus",
  "Disruptor",
  "Phoenix",
  "VoidRay",
  "Oracle",
  "Tempest",
  "Carrier",
  "Mothership",

  // Terran
  "SCV",
  "MULE",
  "Marine",
  "Reaper",
  "Marauder",
  "Ghost",
  "Hellion",
  "WidowMine",
  "Cyclone",
  "SiegeTank",

  // which does thor spawn as?
  "Thor",
  "ThorAP",

  "VikingFighter",
  "Medivac",
  "Liberator",
  "Raven",
  "Banshee",
  "Battlecruiser",

  // Zerg
  "Drone",
  "Overlord",
  "Queen",
  "Zergling",
  "Baneling",
  "Roach",
  "Ravager",
  "Overseer",
  "Hydralisk",
  "LurkerMP",
  "Mutalisk",
  "Corrupter",
  "SwarmHostMP",
  "Infestor",
  "Viper",
  "Ultralisk",
  "BroodLord",
];

// doesn't include supply structures, gas collectors and support structures
const BUILDINGS: [&str; 45] = [
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

  // gas collectors
  "Assimilator",
  "Refinery",
  "Extractor",
];

pub struct ObjectEvent;

pub struct Object<'a> {
  type_name: &'a str,
  object_name: &'a str,
}

const MAX_BUILD_LENGTH: u8 = 15;

impl ObjectEvent {
  pub fn new(game: &mut Game, state: &mut GameState, event: &Event) -> Result<(), &'static str> {
    let mut player_id: u8 = 0;
    let mut object = Object { type_name: "", object_name: "" };
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
        "m_unitTypeName" => if let DecoderResult::Blob(object_name) = value {
          if UNITS.contains(&object_name.as_str()) {
            object.object_name = object_name;
            object.type_name = "unit"
          }

          if BUILDINGS.contains(&object_name.as_str()) {
            object.object_name = object_name;
            object.type_name = "building";
          }
        },
        "m_unitTagIndex" => if let DecoderResult::Value(index) = value {
          tag_index = *index as u32;
        },
        "m_unitTagRecycle" => if let DecoderResult::Value(recycle) = value{
          tag_recycle = *recycle as u32;
        },
        "_gameloop" => if let DecoderResult::Value(gameloop) = value {
          current_gameloop = *gameloop as u16;
        },
        _other => continue,
      }
    }

    if object.object_name == "" {
      return Err("Object name not found");
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

    // if game.builds[player_index as usize].len() < 10 && current_gameloop > 0 {
    //   game.builds[player_index as usize].push(building_name);
    // }

    state.gameloop = current_gameloop;
    let object_type_state = if object.type_name == "building" {
      state.buildings
    } else {
      state.units
    };
    object_type_state
      .entry(object.object_name.to_string())
      .and_modify(|count| *count += 1)
      .or_insert(1);

    // 9408 = ~7min, 22.4 gameloops per sec
    if
      current_gameloop > 0 &&
      current_gameloop > 9408 &&
      object.type_name == "building" &&
      !(object.object_name.contains("Reactor") || object.object_name.contains("TechLab")) &&
      game.builds[player_index as usize].len() < MAX_BUILD_LENGTH as usize
    {
      game.builds[player_index as usize].push(object.object_name.to_string());
    }

    Ok(())
  }
}
