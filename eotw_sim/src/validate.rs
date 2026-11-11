//! O teste de equilíbrio do Prototype 01 (GDD 18.6), na paisagem inteira e
//! com todas as espécies: corre `days` dias em várias seeds e guarda, por
//! seed, o mínimo, a média e o máximo de cada população e da vegetação.
//! `eotw_cli --validate` escreve o resultado numa tabela.

use crate::landscape::Landscape;
use crate::sim::Config;
use crate::ungulates::Species;

/// Mínimo, média e máximo de uma série.
#[derive(Clone, Copy, Debug, Default)]
pub struct Range {
    pub min: f32,
    pub mean: f32,
    pub max: f32,
}

impl Range {
    fn of(values: &[f32]) -> Self {
        let min = values.iter().copied().fold(f32::MAX, f32::min);
        let max = values.iter().copied().fold(f32::MIN, f32::max);
        let mean = values.iter().sum::<f32>() / values.len().max(1) as f32;
        Self { min, mean, max }
    }
}

/// O que se mediu numa seed.
#[derive(Clone, Debug)]
pub struct SeedRun {
    pub seed: u64,
    pub rabbits: Range,
    pub foxes: Range,
    pub deer: Range,
    pub boar: Range,
    pub biomass: Range,
    pub wolf_visits: usize,
    pub wolf_kills: usize,
}

/// A paisagem do jogo (5x5 regiões, o jogador no meio), `days` dias.
pub fn run(seed: u64, days: u64) -> SeedRun {
    let mut l = Landscape::new(Config { seed, ..Config::default() }, 5, 5, (60, 60));
    let mut series = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..days {
        l.run_days(1);
        let (rabbits, foxes) = l.totals();
        series[0].push(rabbits as f32);
        series[1].push(foxes as f32);
        series[2].push(l.animals(Species::Deer) as f32);
        series[3].push(l.animals(Species::Boar) as f32);
        series[4].push(l.mean_biomass());
    }
    SeedRun {
        seed,
        rabbits: Range::of(&series[0]),
        foxes: Range::of(&series[1]),
        deer: Range::of(&series[2]),
        boar: Range::of(&series[3]),
        biomass: Range::of(&series[4]),
        wolf_visits: l.wolf_visits,
        wolf_kills: l.wolf_kills,
    }
}
