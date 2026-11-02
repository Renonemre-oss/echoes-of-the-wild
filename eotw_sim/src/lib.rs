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

pub mod fox;
pub mod rabbit;
pub mod report;
pub mod rng;
pub mod scent;
pub mod spatial;
pub mod sim;
pub mod sweep;
pub mod world;

pub use fox::{Fox, FoxParams};
pub use rabbit::{Rabbit, RabbitParams};
pub use rng::Rng;
pub use scent::{Scent, ScentParams};
pub use sim::{CabinLog, CabinParams, Config, DayStats, MigrationParams, Simulation, MIN_REGION_SIZE, TICKS_PER_DAY};
pub use world::{VegetationParams, World};
