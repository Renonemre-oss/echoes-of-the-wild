//! Simulação agregada de uma região (GDD 7.4 e 7.9): longe do jogador, uma
//! região não tem coelhos nem raposas individuais, só contas por idade, e
//! avança um dia de cada vez.
//!
//! As regras não foram inventadas: vêm das taxas medidas na simulação
//! completa (30 seeds, 400 dias, ver DIARIO.md):
//! - nascimentos por coelho crescem com a vegetação, quase em linha reta;
//! - mortes por fome por coelho caem depressa quando há mais vegetação;
//! - cada raposa come cerca de 0,21 coelhos por dia, com muitos ou poucos
//!   coelhos: a caça é limitada pela fome da raposa, não pelas presas;
//! - as raposas têm poucas crias, e menos quando são mais (travão de
//!   densidade). Tentei tirar isto da própria regra do travão, supondo as
//!   raposas espalhadas ao acaso, mas na simulação completa elas juntam-se
//!   nos sítios com mais coelhos, e com uma só raposa a população recuperava
//!   duas vezes mais depressa do que na completa. A tabela medida acerta, com
//!   uma exceção: uma raposa sozinha não tem travão e tem crias logo.
//!
//! O que a simulação agregada tem de reproduzir é o comportamento da
//! completa: o mesmo equilíbrio e a mesma recuperação depois de um choque.

use crate::rng::Rng;
use crate::sim::{Config, Simulation, TICKS_PER_DAY};

/// O tamanho do mapa (24x24) onde as tabelas foram medidas.
const REFERENCE_CELLS: f32 = 576.0;

/// A maior média que o sorteio de Poisson faz de uma vez (ver `poisson`).
const POISSON_CHUNK: f32 = 30.0;

/// Os números do modelo agregado, medidos na simulação completa.
#[derive(Clone, Debug)]
pub struct AggregateParams {
    /// Nascimentos por coelho adulto por dia, por vegetação média: pares
    /// (vegetação, taxa), medidos na simulação completa; entre pontos
    /// interpola-se em linha reta. Acima de 5 as medições saltam muito (no
    /// arranque os adultos reproduzem-se todos ao mesmo tempo), por isso a
    /// tabela sobe até ao máximo possível: uma cria a cada `repro_cooldown`
    /// (5 dias), ou seja 0,2 por dia.
    pub births: Vec<(f32, f32)>,
    /// Mortes por fome por coelho por dia, por vegetação média.
    pub hunger: Vec<(f32, f32)>,
    /// Coelhos comidos por raposa por dia.
    pub prey_per_fox: f32,
    /// Crias por raposa por dia, pelo número de raposas em cada 24x24 de
    /// região (medido num mapa desse tamanho).
    pub fox_births: Vec<(f32, f32)>,
    /// Vegetação comida por coelho por dia com a vegetação de equilíbrio
    /// (cerca de 4). Com menos vegetação comem menos, com a mesma resposta
    /// funcional do coelho (`half_saturation`). A curva de crescimento diz que,
    /// para a vegetação ficar em 3,85 com 283 coelhos, cada um come 3,46 por
    /// dia; o ajuste à simulação completa deu 3,5.
    pub grazing: f32,
}

impl Default for AggregateParams {
    fn default() -> Self {
        Self {
            births: vec![
                (2.5, 0.0),
                (2.625, 0.0023),
                (2.875, 0.0050),
                (3.125, 0.0065),
                (3.375, 0.0085),
                (3.625, 0.0112),
                (3.875, 0.0143),
                (4.125, 0.0180),
                (4.375, 0.0217),
                (4.625, 0.0307),
                (4.875, 0.0473),
                (6.0, 0.10),
                (7.5, 0.20),
            ],
            hunger: vec![
                (2.75, 0.0338),
                (3.25, 0.0095),
                (3.75, 0.0036),
                (4.25, 0.0013),
                (4.75, 0.0004),
                (5.25, 0.0),
            ],
            prey_per_fox: 0.21,
            fox_births: vec![
                // Sozinha: sem rivais, tem uma cria logo que pode (uma a cada
                // `repro_cooldown`, 20 dias). Com companhia, a taxa medida.
                (1.0, 0.05),
                (2.0, 0.0039),
                (3.5, 0.0054),
                (5.5, 0.0032),
                (7.5, 0.0029),
                (9.5, 0.0016),
                (11.5, 0.0020),
                (13.5, 0.0011),
            ],
            grazing: 3.5,
        }
    }
}

/// Interpola em linha reta numa tabela de pares (x, y) ordenada por x. Fora
/// da tabela, fica com o valor da ponta mais próxima.
fn lookup(table: &[(f32, f32)], x: f32) -> f32 {
    let (first, last) = (table[0], table[table.len() - 1]);
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    let i = table.partition_point(|&(tx, _)| tx <= x);
    let (x0, y0) = table[i - 1];
    let (x1, y1) = table[i];
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

/// Uma região agregada.
#[derive(Clone, Debug)]
pub struct AggregateRegion {
    /// Número de células da região.
    pub cells: usize,
    /// Vegetação média por célula.
    pub biomass: f32,
    /// Coelhos por idade em dias (índice 0 = nascidos hoje).
    pub rabbits: Vec<u32>,
    /// Raposas por idade em dias.
    pub foxes: Vec<u32>,
    pub day: u64,
    config: Config,
    params: AggregateParams,
    rng: Rng,
}

impl AggregateRegion {
    pub fn new(config: Config, params: AggregateParams, seed: u64) -> Self {
        let rabbit_days = (config.rabbit.max_age_ticks / TICKS_PER_DAY) as usize + 1;
        let fox_days = (config.fox.max_age_ticks / TICKS_PER_DAY) as usize + 1;
        Self {
            cells: config.width * config.height,
            biomass: 0.0,
            rabbits: vec![0; rabbit_days],
            foxes: vec![0; fox_days],
            day: 0,
            config,
            params,
            rng: Rng::new(seed),
        }
    }

    /// Agrega uma simulação completa (GDD 7.9, quando o jogador se afasta):
    /// conta os coelhos e as raposas por idade e guarda a vegetação média.
    /// O detalhe individual (posição, energia) perde-se, como a GDD aceita.
    pub fn from_simulation(sim: &Simulation, params: AggregateParams, seed: u64) -> Self {
        let mut region = Self::new(sim.config().clone(), params, seed);
        region.day = sim.day();
        region.biomass = sim.world.mean_biomass();
        let last = region.rabbits.len() - 1;
        for r in &sim.rabbits {
            region.rabbits[((r.age / TICKS_PER_DAY) as usize).min(last)] += 1;
        }
        let last = region.foxes.len() - 1;
        for f in &sim.foxes {
            region.foxes[((f.age / TICKS_PER_DAY) as usize).min(last)] += 1;
        }
        // As raposas que ainda não chegaram já não chegam: numa região agregada
        // o arranque já passou.
        region
    }

    /// Uma região que começa como `Simulation::new`: os coelhos iniciais já
    /// adultos, a vegetação entre 40% e 100% (média 70%), e as raposas a
    /// chegarem no dia `foxes_arrive_day`.
    pub fn like_new_world(config: Config, params: AggregateParams, seed: u64) -> Self {
        let mut region = Self::new(config, params, seed);
        region.biomass = 0.7 * region.config.vegetation.capacity;
        let adult = (region.config.rabbit.maturity_ticks / TICKS_PER_DAY) as usize;
        region.rabbits[adult] = region.config.initial_rabbits as u32;
        if region.config.foxes_arrive_day == 0 {
            region.foxes_arrive();
        }
        region
    }

    fn foxes_arrive(&mut self) {
        let adult = (self.config.fox.maturity_ticks / TICKS_PER_DAY) as usize;
        self.foxes[adult] += self.config.initial_foxes as u32;
    }

    pub fn rabbit_count(&self) -> u32 {
        self.rabbits.iter().sum()
    }

    pub fn fox_count(&self) -> u32 {
        self.foxes.iter().sum()
    }

    /// Um dia de simulação agregada.
    pub fn step_day(&mut self) {
        self.day += 1;
        let p = self.params.clone();
        let b = self.biomass;
        let r = self.rabbit_count() as f32;
        let f = self.fox_count() as f32;

        // Vegetação: o crescimento de um dia (a mesma curva logística da
        // simulação completa, 24 vezes) menos o que os coelhos comem.
        let veg = &self.config.vegetation;
        let mut grown = b;
        for _ in 0..TICKS_PER_DAY {
            let room = 1.0 - grown / veg.capacity;
            grown += (veg.regrowth * grown + veg.seed_growth * veg.capacity) * room;
        }
        // Os coelhos comem o que precisam, mas não mais do que a erva acima de
        // `min_food` (abaixo disso já não vale a pena comer, como na completa).
        let half = self.config.rabbit.half_saturation;
        let appetite = p.grazing * (b / (b + half)) / (4.0 / (4.0 + half));
        let edible = (grown - self.config.rabbit.min_food).max(0.0) * self.cells as f32;
        let eaten = (appetite * r).min(edible) / self.cells as f32;
        self.biomass = (grown - eaten).clamp(0.0, veg.capacity);

        // Coelhos: nascimentos (só dos adultos), fome, caça e velhice.
        let rabbit_adults = adults(&self.rabbits, self.config.rabbit.maturity_ticks) as f32;
        let births = self.poisson(rabbit_adults * lookup(&p.births, b));
        let hunger_rate = lookup(&p.hunger, b);
        self.remove_rabbits_at_random(hunger_rate);
        let hunted = self.poisson(f * p.prey_per_fox);
        self.remove_rabbits_count(hunted);
        age(&mut self.rabbits, births);

        // Raposas: crias (menos quando são mais) e velhice. Sem coelhos que
        // cheguem, morrem de fome.
        // A tabela foi medida num mapa de 24x24, por isso conta raposas por
        // cada 24x24 de região: pelo número absoluto, uma região de 48x48
        // parecia sobrelotada e ficava com 18 raposas em vez de 34.
        let density = f * REFERENCE_CELLS / self.cells as f32;
        let cubs = self.poisson(f * lookup(&p.fox_births, density));
        if r < f * 5.0 {
            self.remove_foxes_at_random(0.05);
        }
        age(&mut self.foxes, cubs);
        if self.day == self.config.foxes_arrive_day {
            self.foxes_arrive();
        }
    }

    /// Um número ao acaso com distribuição de Poisson desta média. O método de
    /// Knuth só funciona com médias pequenas: com f32, `exp(-média)` fica
    /// tão pequeno acima de cerca de 100 que o resultado ficava preso nos 103
    /// (20 000 coelhos adultos davam 103 crias em vez de 434). Uma média
    /// grande parte-se em pedaços de `POISSON_CHUNK`: a soma de Poissons é
    /// uma Poisson, por isso o resultado continua exato.
    fn poisson(&mut self, mean: f32) -> u32 {
        let mut mean = mean;
        let mut k = 0;
        while mean > POISSON_CHUNK {
            k += self.poisson_small(POISSON_CHUNK);
            mean -= POISSON_CHUNK;
        }
        k + self.poisson_small(mean)
    }

    /// O método de Knuth, para médias até `POISSON_CHUNK`.
    fn poisson_small(&mut self, mean: f32) -> u32 {
        if mean <= 0.0 {
            return 0;
        }
        let limit = (-mean).exp();
        let mut k = 0;
        let mut prod = self.rng.next_f32();
        while prod > limit {
            k += 1;
            prod *= self.rng.next_f32();
        }
        k
    }

    fn remove_rabbits_at_random(&mut self, rate: f32) {
        for c in self.rabbits.iter_mut() {
            let mut kept = 0;
            for _ in 0..*c {
                if self.rng.next_f32() >= rate {
                    kept += 1;
                }
            }
            *c = kept;
        }
    }

    fn remove_foxes_at_random(&mut self, rate: f32) {
        for c in self.foxes.iter_mut() {
            let mut kept = 0;
            for _ in 0..*c {
                if self.rng.next_f32() >= rate {
                    kept += 1;
                }
            }
            *c = kept;
        }
    }

    /// Retira `n` coelhos ao acaso, de qualquer idade.
    fn remove_rabbits_count(&mut self, n: u32) {
        for _ in 0..n {
            let total = self.rabbit_count();
            if total == 0 {
                return;
            }
            let mut pick = self.rng.range_usize(total as usize) as u32;
            for c in self.rabbits.iter_mut() {
                if pick < *c {
                    *c -= 1;
                    break;
                }
                pick -= *c;
            }
        }
    }
}

/// Quantos já têm idade para se reproduzir.
fn adults(cohorts: &[u32], maturity_ticks: u64) -> u32 {
    let from = (maturity_ticks / TICKS_PER_DAY) as usize;
    cohorts[from.min(cohorts.len())..].iter().sum()
}

/// Envelhece todos um dia, junta os nascidos com idade 0, e os que passam da
/// idade máxima morrem.
fn age(cohorts: &mut [u32], born: u32) {
    cohorts.rotate_right(1);
    cohorts[0] = born;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aquecida(seed: u64) -> Simulation {
        Simulation::warmed_up(Config {
            seed,
            ..Config::default()
        })
    }

    /// GDD 18.6: na conversão, a população conserva-se.
    #[test]
    fn agregar_conserva_a_populacao() {
        let sim = aquecida(1);
        let r = AggregateRegion::from_simulation(&sim, AggregateParams::default(), 1);
        assert_eq!(r.rabbit_count() as usize, sim.rabbits.len());
        assert_eq!(r.fox_count() as usize, sim.foxes.len());
        assert!((r.biomass - sim.world.mean_biomass()).abs() < 1e-6);
        assert_eq!(r.day, sim.day());
    }

    #[test]
    fn a_mesma_seed_da_o_mesmo_resultado() {
        let sim = aquecida(2);
        let correr = || {
            let mut r = AggregateRegion::from_simulation(&sim, AggregateParams::default(), 7);
            (0..100)
                .map(|_| {
                    r.step_day();
                    (r.rabbit_count(), r.fox_count())
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(correr(), correr());
    }

    /// Média de coelhos, raposas e vegetação nos dias 40 a 300 depois do fim
    /// do aquecimento, em `seeds` seeds, nas duas simulações, num mapa de
    /// `lado` x `lado` com a densidade de animais e tocas do de 24x24.
    fn medias(seeds: u64, lado: usize) -> ([f32; 3], [f32; 3]) {
        let (mut completa, mut agregada) = ([0.0f32; 3], [0.0f32; 3]);
        let mut n = 0.0;
        for seed in 1..=seeds {
            let mut config = Config {
                seed,
                ..Config::default()
            };
            crate::sweep::sized_map(&mut config, lado);
            config.initial_foxes = (config.initial_foxes * lado * lado / 576).max(1);
            let mut sim = Simulation::warmed_up(config);
            let mut r = AggregateRegion::from_simulation(&sim, AggregateParams::default(), seed);
            for dia in 0..300 {
                sim.run_days(1);
                r.step_day();
                if dia >= 40 {
                    completa[0] += sim.rabbits.len() as f32;
                    completa[1] += sim.foxes.len() as f32;
                    completa[2] += sim.world.mean_biomass();
                    agregada[0] += r.rabbit_count() as f32;
                    agregada[1] += r.fox_count() as f32;
                    agregada[2] += r.biomass;
                    n += 1.0;
                }
            }
        }
        (completa.map(|v| v / n), agregada.map(|v| v / n))
    }

    /// A simulação agregada tem o mesmo equilíbrio que a completa, a partir
    /// do mesmo estado (o fim do aquecimento): é isso que deixa trocar uma
    /// pela outra quando o jogador se afasta.
    #[test]
    fn a_agregada_tem_o_mesmo_equilibrio_que_a_completa() {
        let (c, a) = medias(10, 24);
        let perto = |x: f32, y: f32, tol: f32| (x - y).abs() <= tol * x;
        assert!(perto(c[0], a[0], 0.08), "coelhos: completa {:.0}, agregada {:.0}", c[0], a[0]);
        assert!(perto(c[1], a[1], 0.25), "raposas: completa {:.1}, agregada {:.1}", c[1], a[1]);
        assert!(perto(c[2], a[2], 0.05), "vegetação: completa {:.2}, agregada {:.2}", c[2], a[2]);
    }

    /// As tabelas foram medidas num mapa de 24x24, mas uma região pode ser
    /// maior. Contadas pelo número de raposas e não pela densidade, uma região
    /// de 48x48 ficava com 18 raposas em vez de 34.
    #[test]
    fn uma_regiao_maior_tem_o_mesmo_equilibrio() {
        let (c, a) = medias(4, 48);
        let perto = |x: f32, y: f32, tol: f32| (x - y).abs() <= tol * x;
        assert!(perto(c[0], a[0], 0.10), "coelhos: completa {:.0}, agregada {:.0}", c[0], a[0]);
        assert!(perto(c[1], a[1], 0.25), "raposas: completa {:.1}, agregada {:.1}", c[1], a[1]);
    }

    /// O sorteio de Poisson acerta na média mesmo com médias grandes. Antes
    /// ficava preso nos 103: 20 000 coelhos adultos davam 103 crias por dia.
    #[test]
    fn poisson_com_media_grande() {
        let mut r = AggregateRegion::new(Config::default(), AggregateParams::default(), 1);
        for media in [0.5f32, 5.0, 50.0, 434.0, 2000.0] {
            let n = 200;
            let soma: u32 = (0..n).map(|_| r.poisson(media)).sum();
            let obtida = soma as f32 / n as f32;
            // A média de 200 sorteios fica a menos de 4 desvios-padrão.
            let margem = 4.0 * (media / n as f32).sqrt();
            assert!((obtida - media).abs() < margem, "média {media}: obtida {obtida}");
        }
    }

    /// Um choque que o modelo nunca viu ao ser ajustado: metade dos coelhos
    /// desaparece. Nas duas simulações, ao fim de 21 dias já recuperaram.
    #[test]
    fn a_agregada_recupera_de_um_choque_como_a_completa() {
        let (mut completa, mut agregada) = (0.0, 0.0);
        for seed in 1..=10 {
            let mut sim = aquecida(seed);
            let mut r = AggregateRegion::from_simulation(&sim, AggregateParams::default(), seed);
            let mut k = 0;
            sim.rabbits.retain(|_| {
                k += 1;
                k % 2 == 0
            });
            r.remove_rabbits_count(r.rabbit_count() / 2);
            sim.run_days(21);
            for _ in 0..21 {
                r.step_day();
            }
            completa += sim.rabbits.len() as f32;
            agregada += r.rabbit_count() as f32;
        }
        let (completa, agregada) = (completa / 10.0, agregada / 10.0);
        assert!(completa > 250.0 && agregada > 250.0, "completa {completa:.0}, agregada {agregada:.0}");
        assert!((completa - agregada).abs() < 0.15 * completa, "completa {completa:.0}, agregada {agregada:.0}");
    }
}
