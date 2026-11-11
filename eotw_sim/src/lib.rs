//! Simulação do ecossistema de Echoes of the Wild.
//!
//! Esta biblioteca NÃO depende de nenhum motor gráfico. Assim:
//! - corre em modo texto (`eotw_cli`) e pode ser testada automaticamente;
//! - o jogo (`eotw_game`, com Bevy) limita-se a desenhar o que a simulação calcula.
//!
//! Regras de ouro (GDD, secções 17.8 e 20.13):
//! - passo de tempo fixo (1 tick = 1 hora de jogo);
//! - sem aleatoriedade global: tudo vem do `Rng` com seed;
//! - a mesma seed dá sempre o mesmo resultado.

pub mod calendar;
pub mod daylight;
pub mod fox;
pub mod landscape;
pub mod noise;
pub mod rabbit;
pub mod region;
pub mod resources;
pub mod report;
pub mod river;
pub mod rng;
pub mod scent;
pub mod spatial;
pub mod shelter;
pub mod sim;
pub mod survival;
pub mod sweep;
pub mod ungulates;
pub mod validate;
pub mod wolves;
pub mod world;

pub use calendar::{Breeding, CalendarParams, Climate, Date, FoxPhase, Season};
pub use daylight::DaylightParams;
pub use fox::{Fox, FoxParams};
pub use noise::NoiseParams;
pub use rabbit::{Rabbit, RabbitParams};
pub use resources::{Craft, Inventory, Item, Resource, ResourceKind, ResourceParams, Tool};
pub use river::RiverParams;
pub use rng::Rng;
pub use scent::{Scent, ScentParams};
pub use shelter::{Build, Shelter, ShelterParams, Stage};
pub use sim::{CabinLog, CabinParams, Config, DayStats, MigrationParams, Simulation, MIN_REGION_SIZE, SPAWN_CLEAR_RADIUS, TICKS_PER_DAY};
pub use survival::{Needs, NeedsParams};
pub use ungulates::{Herd, Species, UngulateParams};
pub use wolves::{Pack, PackEvent, PackParams};
pub use world::{VegetationParams, World};
