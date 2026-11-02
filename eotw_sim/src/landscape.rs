//! A paisagem (GDD 7.4 e 7.9): um mundo feito de várias regiões de 24x24.
//!
//! À volta do jogador há uma janela de 3x3 regiões simuladas por inteiro,
//! numa só `Simulation`, para os animais passarem de uma região para a outra
//! sem costuras. As outras regiões são agregadas (`AggregateRegion`).
//!
//! Quando o jogador muda de região, a janela desloca-se:
//! - as regiões que saem da janela são agregadas;
//! - as que entram são expandidas;
//! - as que ficam guardam cada animal tal como estava, e também a vegetação
//!   e o cheiro.
//!
//! O jogador está sempre na região do meio, por isso a margem da janela, onde
//! as regiões entram e saem, fica a 24 células ou mais dele: nada aparece nem
//! desaparece à frente dele.
//!
//! Uma vez por dia, alguns animais passam para as regiões vizinhas, à taxa
//! medida na completa (`rabbits_cross` e `foxes_cross`). É isso que deixa uma
//! região vazia voltar a ter coelhos e raposas.

use crate::fox::Fox;
use crate::rabbit::Rabbit;
use crate::region::{AggregateParams, AggregateRegion};
use crate::rng::Rng;
use crate::scent::KINDS;
use crate::sim::{Config, Simulation, MIN_REGION_SIZE, SPAWN_CLEAR_RADIUS, TICKS_PER_DAY};
use crate::world::{distance, World};

/// O lado de uma região, em células.
pub const REGION: usize = MIN_REGION_SIZE;

/// Quantas regiões há à volta da do jogador, para cada lado, na janela.
const WINDOW_REACH: usize = 1;

/// Os animais que passam da janela para uma região agregada saem desta faixa
/// junto à fronteira, e os que chegam aparecem nela.
const BORDER_STRIP: usize = 4;

/// As sequências de números ao acaso da paisagem (ver `Rng::derived`).
const STREAM_START: u64 = 11;
const STREAM_REGION: u64 = 12;
const STREAM_WINDOW: u64 = 13;
const STREAM_CROSSING: u64 = 14;

/// O que existe numa região cortada da janela: o terreno, o cheiro e os
/// animais, em coordenadas da região (0 a 23).
struct Cut {
    world: World,
    scent: Vec<[f32; KINDS]>,
    rabbits: Vec<Rabbit>,
    foxes: Vec<Fox>,
}

/// A janela completa: que regiões cobre e a simulação delas.
struct Window {
    /// A região do canto de cima à esquerda, e quantas regiões tem de lado.
    rx: usize,
    ry: usize,
    cols: usize,
    rows: usize,
    sim: Simulation,
}

impl Window {
    fn contains(&self, rx: usize, ry: usize) -> bool {
        (self.rx..self.rx + self.cols).contains(&rx) && (self.ry..self.ry + self.rows).contains(&ry)
    }
}

pub struct Landscape {
    pub cols: usize,
    pub rows: usize,
    /// A configuração de uma região (24x24).
    config: Config,
    params: AggregateParams,
    /// As regiões agregadas, por `ry * cols + rx`. `None` nas da janela.
    regions: Vec<Option<AggregateRegion>>,
    window: Window,
    /// O jogador, em células da paisagem inteira.
    player: (usize, usize),
    rng: Rng,
    /// Acontecimentos das simulações completas que já foram desfeitas,
    /// para não se perderem quando a janela se desloca.
    pub events: Vec<String>,
    /// Quantos animais passaram de uma região para outra, desde o início.
    pub crossings: usize,
}

impl Landscape {
    /// Uma paisagem de `cols` x `rows` regiões, com o jogador em `player`.
    /// Cada região começa como um mundo de 24x24 aquecido, com a sua própria
    /// seed, e é logo agregada; depois expande-se a janela à volta do jogador.
    pub fn new(config: Config, cols: usize, rows: usize, player: (usize, usize)) -> Self {
        let config = Config {
            width: REGION,
            height: REGION,
            cabin: None,
            ..config
        };
        let params = AggregateParams::default();
        let regions = (0..cols * rows)
            .map(|id| {
                let seed = Rng::derived(&[config.seed, id as u64, STREAM_START]).next_u64();
                let sim = Simulation::warmed_up(Config {
                    seed,
                    ..config.clone()
                });
                Some(AggregateRegion::from_simulation(&sim, params.clone(), seed))
            })
            .collect();
        let player = (player.0.min(cols * REGION - 1), player.1.min(rows * REGION - 1));
        let rng = Rng::derived(&[config.seed, STREAM_CROSSING]);
        // Uma janela vazia, só para a estrutura existir; `open_window` faz a verdadeira.
        let empty = Window {
            rx: 0,
            ry: 0,
            cols: 0,
            rows: 0,
            sim: Simulation::from_parts(
                config.clone(),
                Rng::new(0),
                World::new(1, 1, config.vegetation.clone(), &mut Rng::new(0)),
                Vec::new(),
                Vec::new(),
                0,
            ),
        };
        let mut landscape = Self {
            cols,
            rows,
            config,
            params,
            regions,
            window: empty,
            player,
            rng,
            events: Vec::new(),
            crossings: 0,
        };
        landscape.open_window();
        landscape
    }

    /// A simulação completa da janela (para desenhar).
    pub fn window_sim(&self) -> &Simulation {
        &self.window.sim
    }

    /// A célula da paisagem onde começa a janela.
    pub fn window_origin(&self) -> (usize, usize) {
        (self.window.rx * REGION, self.window.ry * REGION)
    }

    pub fn player(&self) -> (usize, usize) {
        self.player
    }

    pub fn day(&self) -> u64 {
        self.window.sim.day()
    }

    /// Uma região agregada (`None` se estiver na janela).
    pub fn region(&self, rx: usize, ry: usize) -> Option<&AggregateRegion> {
        self.regions[ry * self.cols + rx].as_ref()
    }

    pub fn region_mut(&mut self, rx: usize, ry: usize) -> Option<&mut AggregateRegion> {
        self.regions[ry * self.cols + rx].as_mut()
    }

    /// Coelhos e raposas numa região, esteja ela agregada ou na janela.
    pub fn counts(&self, rx: usize, ry: usize) -> (usize, usize) {
        if let Some(r) = self.region(rx, ry) {
            return (r.rabbit_count() as usize, r.fox_count() as usize);
        }
        let (ox, oy) = self.window_origin();
        let inside = |x: usize, y: usize| (ox + x) / REGION == rx && (oy + y) / REGION == ry;
        let sim = &self.window.sim;
        (
            sim.rabbits.iter().filter(|a| inside(a.x, a.y)).count(),
            sim.foxes.iter().filter(|f| inside(f.x, f.y)).count(),
        )
    }

    /// Coelhos e raposas na paisagem inteira.
    pub fn totals(&self) -> (usize, usize) {
        let mut total = (self.window.sim.rabbits.len(), self.window.sim.foxes.len());
        for r in self.regions.iter().flatten() {
            total.0 += r.rabbit_count() as usize;
            total.1 += r.fox_count() as usize;
        }
        total
    }

    /// Anda uma célula na direção (dx, dy), sem sair da paisagem. Se o
    /// jogador mudar de região, a janela desloca-se com ele.
    pub fn move_player(&mut self, dx: i32, dy: i32) {
        let (x, y) = self.player;
        let nx = (x as i32 + dx).clamp(0, (self.cols * REGION) as i32 - 1) as usize;
        let ny = (y as i32 + dy).clamp(0, (self.rows * REGION) as i32 - 1) as usize;
        self.player = (nx, ny);
        if (x / REGION, y / REGION) != (nx / REGION, ny / REGION) {
            self.open_window();
        } else {
            self.place_player_in_window();
        }
    }

    /// Um tick. No fim de cada dia avançam as regiões agregadas e há
    /// animais a mudar de região.
    pub fn step(&mut self) {
        self.window.sim.step();
        if self.window.sim.tick() % TICKS_PER_DAY == 0 {
            for r in self.regions.iter_mut().flatten() {
                r.step_day();
            }
            self.cross_borders();
        }
    }

    pub fn run_days(&mut self, days: u64) {
        for _ in 0..days * TICKS_PER_DAY {
            self.step();
        }
    }

    fn place_player_in_window(&mut self) {
        let (ox, oy) = self.window_origin();
        self.window.sim.place_player(Some((self.player.0 - ox, self.player.1 - oy)));
    }

    /// As regiões da janela à volta do jogador: a dele e as vizinhas que
    /// existem.
    fn window_bounds(&self) -> (usize, usize, usize, usize) {
        let (px, py) = (self.player.0 / REGION, self.player.1 / REGION);
        let rx = px.saturating_sub(WINDOW_REACH);
        let ry = py.saturating_sub(WINDOW_REACH);
        let rx1 = (px + WINDOW_REACH).min(self.cols - 1);
        let ry1 = (py + WINDOW_REACH).min(self.rows - 1);
        (rx, ry, rx1 - rx + 1, ry1 - ry + 1)
    }

    /// Põe a janela à volta do jogador: corta a janela antiga em regiões,
    /// agrega as que saem, expande as que entram e junta tudo numa
    /// simulação nova.
    fn open_window(&mut self) {
        let (rx, ry, cols, rows) = self.window_bounds();
        let day = self.window.sim.day().max(self.any_region_day());
        let tick = self.window.sim.tick().max(day * TICKS_PER_DAY);
        self.events.append(&mut self.window.sim.events);

        // 1. As regiões da janela antiga: as que ficam guardam-se como
        //    estão; as que saem são agregadas.
        let mut kept: Vec<((usize, usize), Cut)> = Vec::new();
        for oy in 0..self.window.rows {
            for ox in 0..self.window.cols {
                let (gx, gy) = (self.window.rx + ox, self.window.ry + oy);
                let cut = self.cut(ox, oy);
                if (rx..rx + cols).contains(&gx) && (ry..ry + rows).contains(&gy) {
                    kept.push(((gx, gy), cut));
                } else {
                    let id = gy * self.cols + gx;
                    let seed = Rng::derived(&[self.config.seed, id as u64, day, STREAM_REGION]).next_u64();
                    self.regions[id] = Some(AggregateRegion::from_animals(
                        self.config.clone(),
                        self.params.clone(),
                        seed,
                        day,
                        &cut.world,
                        &cut.rabbits,
                        &cut.foxes,
                    ));
                }
            }
        }

        // 2. Junta a janela nova, região a região.
        let (w, h) = (cols * REGION, rows * REGION);
        let window_config = Config {
            width: w,
            height: h,
            ..self.config.clone()
        };
        let mut rng = Rng::derived(&[self.config.seed, tick, STREAM_WINDOW]);
        let mut world = World::new(w, h, self.config.vegetation.clone(), &mut rng);
        let mut rabbits = Vec::new();
        let mut foxes = Vec::new();
        let mut scent = Vec::new();
        let player = self.player;
        for wy in 0..rows {
            for wx in 0..cols {
                let (gx, gy) = (rx + wx, ry + wy);
                let id = gy * self.cols + gx;
                let cut = match kept.iter().position(|(g, _)| *g == (gx, gy)) {
                    Some(i) => kept.swap_remove(i).1,
                    None => {
                        let region = self.regions[id].take().expect("região fora da janela sem estado agregado");
                        // Nada aparece perto do jogador, mesmo que ele esteja
                        // do outro lado da fronteira.
                        let (ox, oy) = (gx * REGION, gy * REGION);
                        let clear = |c: (usize, usize)| distance((ox + c.0, oy + c.1), player) > SPAWN_CLEAR_RADIUS;
                        let sim = region.expand_with(id as u64, &clear);
                        Cut {
                            scent: vec![[0.0; KINDS]; REGION * REGION],
                            world: sim.world,
                            rabbits: sim.rabbits,
                            foxes: sim.foxes,
                        }
                    }
                };
                let (ox, oy) = (wx * REGION, wy * REGION);
                for y in 0..REGION {
                    for x in 0..REGION {
                        world.set_biomass(ox + x, oy + y, cut.world.biomass_at(x, y));
                        scent.push(((ox + x, oy + y), cut.scent[y * REGION + x]));
                    }
                }
                for &(x, y) in cut.world.burrows() {
                    world.add_burrow(ox + x, oy + y);
                }
                rabbits.extend(cut.rabbits.into_iter().map(|mut a| {
                    (a.x, a.y) = (ox + a.x, oy + a.y);
                    a
                }));
                foxes.extend(cut.foxes.into_iter().map(|mut f| {
                    (f.x, f.y) = (ox + f.x, oy + f.y);
                    f
                }));
            }
        }
        for ((x, y), v) in scent {
            world.scent.set_cell(x, y, v);
        }
        let sim = Simulation::from_parts(window_config, rng, world, rabbits, foxes, tick);
        self.window = Window { rx, ry, cols, rows, sim };
        self.place_player_in_window();
    }

    /// O dia das regiões agregadas (todas têm o mesmo).
    fn any_region_day(&self) -> u64 {
        self.regions.iter().flatten().map(|r| r.day).max().unwrap_or(0)
    }

    /// A região (ox, oy) da janela, em coordenadas dela.
    fn cut(&self, ox: usize, oy: usize) -> Cut {
        let sim = &self.window.sim;
        let (x0, y0) = (ox * REGION, oy * REGION);
        let inside = |x: usize, y: usize| (x0..x0 + REGION).contains(&x) && (y0..y0 + REGION).contains(&y);
        let mut world = World::new(REGION, REGION, self.config.vegetation.clone(), &mut Rng::new(0));
        let mut scent = Vec::with_capacity(REGION * REGION);
        for y in 0..REGION {
            for x in 0..REGION {
                world.set_biomass(x, y, sim.world.biomass_at(x0 + x, y0 + y));
                scent.push(sim.world.scent.cell(x0 + x, y0 + y));
            }
        }
        for &(x, y) in sim.world.burrows() {
            if inside(x, y) {
                world.add_burrow(x - x0, y - y0);
            }
        }
        let rabbits = sim
            .rabbits
            .iter()
            .filter(|a| inside(a.x, a.y))
            .map(|a| Rabbit { x: a.x - x0, y: a.y - y0, ..a.clone() })
            .collect();
        let foxes = sim
            .foxes
            .iter()
            .filter(|f| inside(f.x, f.y))
            .map(|f| Fox { x: f.x - x0, y: f.y - y0, ..f.clone() })
            .collect();
        Cut { world, scent, rabbits, foxes }
    }

    /// As regiões vizinhas de (rx, ry) que existem: cima, baixo, esquerda, direita.
    fn neighbors(&self, rx: usize, ry: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(4);
        if ry > 0 {
            out.push((rx, ry - 1));
        }
        if ry + 1 < self.rows {
            out.push((rx, ry + 1));
        }
        if rx > 0 {
            out.push((rx - 1, ry));
        }
        if rx + 1 < self.cols {
            out.push((rx + 1, ry));
        }
        out
    }

    /// Uma vez por dia: cada animal de uma região agregada passa para cada
    /// região vizinha com a probabilidade medida. Os da janela passam pelas
    /// fronteiras com regiões agregadas (dentro da janela já andam sozinhos).
    fn cross_borders(&mut self) {
        // Primeiro decide-se quem sai, a partir de como está tudo agora, e só
        // depois se põe cada um no destino: assim ninguém muda duas vezes.
        // (idade em ticks, destino, é raposa)
        let mut moving: Vec<(u64, (usize, usize), bool)> = Vec::new();
        for ry in 0..self.rows {
            for rx in 0..self.cols {
                let near = self.neighbors(rx, ry);
                if self.window.contains(rx, ry) {
                    let outside: Vec<_> = near.into_iter().filter(|&(x, y)| !self.window.contains(x, y)).collect();
                    for to in outside {
                        self.leave_window((rx, ry), to, &mut moving);
                    }
                    continue;
                }
                let id = ry * self.cols + rx;
                let mut region = self.regions[id].take().expect("região agregada");
                let p = self.params.clone();
                for (cohorts, chance, fox) in [(&mut region.rabbits, p.rabbits_cross, false), (&mut region.foxes, p.foxes_cross, true)] {
                    for (days, c) in cohorts.iter_mut().enumerate() {
                        let mut stay = 0;
                        for _ in 0..*c {
                            let roll = self.rng.next_f32();
                            let k = (roll / chance) as usize;
                            if k < near.len() {
                                moving.push((days as u64 * TICKS_PER_DAY, near[k], fox));
                            } else {
                                stay += 1;
                            }
                        }
                        *c = stay;
                    }
                }
                self.regions[id] = Some(region);
            }
        }
        self.crossings += moving.len();
        for (age, (tx, ty), fox) in moving {
            match self.regions[ty * self.cols + tx].as_mut() {
                Some(r) if fox => r.add_fox(age),
                Some(r) => r.add_rabbit(age),
                None => self.enter_window((tx, ty), age, fox),
            }
        }
    }

    /// Os animais da região `from` da janela que passam para a região
    /// agregada `to`: tantos como sairiam de uma região agregada, escolhidos
    /// entre os que estão na faixa junto a essa fronteira e longe do jogador.
    fn leave_window(&mut self, from: (usize, usize), to: (usize, usize), moving: &mut Vec<(u64, (usize, usize), bool)>) {
        let (ox, oy) = self.window_origin();
        let (x0, y0) = (from.0 * REGION - ox, from.1 * REGION - oy);
        let player = self.player;
        // Na faixa junto à fronteira com `to`, em coordenadas da janela.
        let strip = |x: usize, y: usize| {
            let (lx, ly) = (x.wrapping_sub(x0), y.wrapping_sub(y0));
            if lx >= REGION || ly >= REGION {
                return false;
            }
            let by_border = match (to.0 as i64 - from.0 as i64, to.1 as i64 - from.1 as i64) {
                (0, -1) => ly < BORDER_STRIP,
                (0, 1) => ly >= REGION - BORDER_STRIP,
                (-1, 0) => lx < BORDER_STRIP,
                _ => lx >= REGION - BORDER_STRIP,
            };
            by_border && distance((ox + x, oy + y), player) > SPAWN_CLEAR_RADIUS
        };
        let p = self.params.clone();
        let in_region = |x: usize, y: usize| x.wrapping_sub(x0) < REGION && y.wrapping_sub(y0) < REGION;
        let sim = &mut self.window.sim;

        let n = sim.rabbits.iter().filter(|a| in_region(a.x, a.y)).count();
        let leaving = binomial(&mut self.rng, n, p.rabbits_cross);
        let mut candidates: Vec<usize> = (0..sim.rabbits.len()).filter(|&i| strip(sim.rabbits[i].x, sim.rabbits[i].y)).collect();
        let mut gone = vec![false; sim.rabbits.len()];
        for _ in 0..leaving.min(candidates.len()) {
            let i = candidates.swap_remove(self.rng.range_usize(candidates.len()));
            gone[i] = true;
            moving.push((sim.rabbits[i].age, to, false));
        }
        let mut k = 0;
        sim.rabbits.retain(|_| {
            k += 1;
            !gone[k - 1]
        });

        let n = sim.foxes.iter().filter(|f| in_region(f.x, f.y)).count();
        let leaving = binomial(&mut self.rng, n, p.foxes_cross);
        let mut candidates: Vec<usize> = (0..sim.foxes.len()).filter(|&i| strip(sim.foxes[i].x, sim.foxes[i].y)).collect();
        let mut gone = vec![false; sim.foxes.len()];
        for _ in 0..leaving.min(candidates.len()) {
            let i = candidates.swap_remove(self.rng.range_usize(candidates.len()));
            gone[i] = true;
            moving.push((sim.foxes[i].age, to, true));
        }
        let mut k = 0;
        sim.foxes.retain(|_| {
            k += 1;
            !gone[k - 1]
        });
    }

    /// Um animal vindo de uma região agregada entra na região `to` da janela,
    /// na faixa junto à fronteira por onde veio. Se não houver lugar longe do
    /// jogador, fica numa célula qualquer da faixa mais afastada dele.
    fn enter_window(&mut self, to: (usize, usize), age: u64, fox: bool) {
        // De que lado vem: o vizinho agregado mais perto. Procura-se uma
        // célula na faixa de cada fronteira com uma região agregada.
        let (ox, oy) = self.window_origin();
        let (x0, y0) = (to.0 * REGION - ox, to.1 * REGION - oy);
        let sides: Vec<(usize, usize)> = self
            .neighbors(to.0, to.1)
            .into_iter()
            .filter(|&(x, y)| !self.window.contains(x, y))
            .collect();
        let side = sides[self.rng.range_usize(sides.len())];
        let along = self.rng.range_usize(REGION);
        let depth = self.rng.range_usize(BORDER_STRIP);
        let (lx, ly) = match (side.0 as i64 - to.0 as i64, side.1 as i64 - to.1 as i64) {
            (0, -1) => (along, depth),
            (0, 1) => (along, REGION - 1 - depth),
            (-1, 0) => (depth, along),
            _ => (REGION - 1 - depth, along),
        };
        let cell = (x0 + lx, y0 + ly);
        if distance((ox + cell.0, oy + cell.1), self.player) <= SPAWN_CLEAR_RADIUS {
            // Não aparece à frente do jogador: fica na região de onde vinha.
            let r = self.regions[side.1 * self.cols + side.0].as_mut().expect("vizinho agregado");
            if fox {
                r.add_fox(age);
            } else {
                r.add_rabbit(age);
            }
            return;
        }
        let days = (age / TICKS_PER_DAY) as usize;
        let template = self.regions[side.1 * self.cols + side.0].as_ref().expect("vizinho agregado");
        if fox {
            let f = template.new_fox(&mut self.rng, days, cell);
            self.window.sim.foxes.push(f);
        } else {
            let a = template.new_rabbit(&mut self.rng, days, cell);
            self.window.sim.rabbits.push(a);
        }
    }
}

/// Quantos de `n` acontecem, cada um com probabilidade `p`.
fn binomial(rng: &mut Rng, n: usize, p: f32) -> usize {
    (0..n).filter(|_| rng.next_f32() < p).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paisagem(seed: u64, lado: usize, jogador: (usize, usize)) -> Landscape {
        Landscape::new(
            Config {
                seed,
                ..Config::default()
            },
            lado,
            lado,
            jogador,
        )
    }

    /// Os animais da janela, em células da paisagem, com o que os distingue.
    fn animais(l: &Landscape) -> Vec<(usize, usize, u64, u32, bool)> {
        let (ox, oy) = l.window_origin();
        let sim = l.window_sim();
        let mut v: Vec<_> = sim
            .rabbits
            .iter()
            .map(|a| (ox + a.x, oy + a.y, a.age, a.energy.to_bits(), false))
            .chain(sim.foxes.iter().map(|f| (ox + f.x, oy + f.y, f.age, f.energy.to_bits(), true)))
            .collect();
        v.sort();
        v
    }

    /// Quando a janela se desloca, nenhum animal se perde nem aparece, e os
    /// das regiões que ficam na janela continuam exatamente iguais.
    #[test]
    fn a_janela_desloca_sem_perder_animais() {
        let mut l = paisagem(1, 4, (47, 36));
        l.run_days(5);
        let antes = l.totals();
        let (ox, oy) = l.window_origin();
        // As regiões (1..=2, 0..=2) ficam na janela quando o jogador passa da
        // região 1 para a 2.
        let ficam = |a: &(usize, usize, u64, u32, bool)| (REGION..3 * REGION).contains(&a.0) && a.1 < 3 * REGION;
        let ficavam: Vec<_> = animais(&l).into_iter().filter(ficam).collect();
        assert_eq!((ox, oy), (0, 0));
        l.move_player(1, 0);
        assert_eq!(l.window_origin(), (REGION, 0), "a janela devia ter andado uma região");
        assert_eq!(l.totals(), antes);
        let ficaram: Vec<_> = animais(&l).into_iter().filter(ficam).collect();
        assert_eq!(ficaram, ficavam);
    }

    /// GDD 7.9: ao andar pela paisagem, nunca aparece nem desaparece um
    /// animal perto do jogador quando a janela se desloca.
    #[test]
    fn nada_aparece_nem_desaparece_perto_do_jogador() {
        let mut l = paisagem(2, 4, (36, 36));
        // Os animais a `SPAWN_CLEAR_RADIUS` ou menos de uma célula.
        let perto = |l: &Landscape, p: (usize, usize)| -> Vec<_> {
            animais(l).into_iter().filter(|a| distance((a.0, a.1), p) <= SPAWN_CLEAR_RADIUS).collect()
        };
        // Uma volta que atravessa várias fronteiras, nos dois sentidos.
        let passos = [(1, 0); 30].into_iter().chain([(0, 1); 30]).chain([(-1, 0); 45]).chain([(0, -1); 40]);
        let mut deslocamentos = 0;
        for (dx, dy) in passos {
            let janela = l.window_origin();
            let p = l.player();
            let destino = ((p.0 as i32 + dx) as usize, (p.1 as i32 + dy) as usize);
            // À volta de onde o jogador vai ficar, antes e depois do passo.
            let antes = perto(&l, destino);
            l.move_player(dx, dy);
            assert_eq!(l.player(), destino);
            if l.window_origin() != janela {
                deslocamentos += 1;
                assert_eq!(perto(&l, destino), antes, "mudou o que está perto do jogador em {destino:?}");
            }
            l.step();
        }
        assert!(deslocamentos >= 4, "a volta devia ter deslocado a janela várias vezes ({deslocamentos})");
    }

    /// A mesma seed e os mesmos passos dão a mesma paisagem.
    #[test]
    fn a_mesma_seed_da_a_mesma_paisagem() {
        let correr = || {
            let mut l = paisagem(3, 3, (30, 30));
            let mut v = Vec::new();
            for d in 0..20 {
                if d == 10 {
                    for _ in 0..20 {
                        l.move_player(1, 0);
                    }
                }
                l.run_days(1);
                v.push(l.totals());
            }
            v
        };
        assert_eq!(correr(), correr());
    }

    /// A paisagem tem, em cada região, o mesmo equilíbrio que uma região
    /// simulada sozinha (cerca de 280 coelhos), seja na janela ou agregada.
    #[test]
    fn a_paisagem_fica_em_equilibrio() {
        let mut janela = (0.0, 0.0);
        let mut agregadas = (0.0, 0.0);
        let mut n = (0.0, 0.0);
        for seed in 1..=3 {
            // 3x3 regiões, o jogador num canto: a janela tem 2x2 regiões e
            // há 5 agregadas.
            let mut l = paisagem(seed, 3, (5, 5));
            for dia in 0..200 {
                l.run_days(1);
                if dia < 40 {
                    continue;
                }
                for ry in 0..3 {
                    for rx in 0..3 {
                        let (r, f) = l.counts(rx, ry);
                        if l.region(rx, ry).is_some() {
                            agregadas.0 += r as f32;
                            agregadas.1 += f as f32;
                            n.1 += 1.0;
                        } else {
                            janela.0 += r as f32;
                            janela.1 += f as f32;
                            n.0 += 1.0;
                        }
                    }
                }
            }
            assert_eq!(l.window_sim().migrations_rabbits + l.window_sim().migrations_foxes, 0, "seed {seed}");
        }
        let (rj, fj) = (janela.0 / n.0, janela.1 / n.0);
        let (ra, fa) = (agregadas.0 / n.1, agregadas.1 / n.1);
        for (onde, r, f) in [("janela", rj, fj), ("agregadas", ra, fa)] {
            assert!((240.0..330.0).contains(&r), "{onde}: {r:.0} coelhos por região");
            assert!((3.0..12.0).contains(&f), "{onde}: {f:.1} raposas por região");
        }
    }

    /// Uma região que fica vazia volta a ter coelhos e raposas, vindos das
    /// vizinhas. Sem passagem entre regiões, ficava vazia para sempre.
    #[test]
    fn uma_regiao_vazia_e_recolonizada() {
        let mut com_raposas = 0;
        for seed in 1..=4 {
            let mut l = paisagem(seed, 3, (5, 5));
            let r = l.region_mut(2, 2).expect("agregada");
            r.rabbits.iter_mut().for_each(|c| *c = 0);
            r.foxes.iter_mut().for_each(|c| *c = 0);
            l.run_days(150);
            let (coelhos, raposas) = l.counts(2, 2);
            assert!(coelhos > 150, "seed {seed}: só {coelhos} coelhos");
            if raposas > 0 {
                com_raposas += 1;
            }
            assert!(l.crossings > 0);
        }
        assert!(com_raposas >= 2, "as raposas voltaram em {com_raposas} de 4 seeds");
    }
}
