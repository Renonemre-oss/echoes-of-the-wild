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

use crate::fox::Fox;
use crate::rabbit::Rabbit;
use crate::resources::Resource;
use crate::rng::Rng;
use crate::sim::{far_from, Config, Simulation, TICKS_PER_DAY};
use crate::world::{distance, World};

/// O tamanho do mapa (24x24) onde as tabelas foram medidas.
const REFERENCE_CELLS: f32 = 576.0;

/// A maior média que o sorteio de Poisson faz de uma vez (ver `poisson`).
const POISSON_CHUNK: f32 = 30.0;

/// As sequências de números ao acaso de cada parte da conversão (ver
/// `Rng::derived`): o terreno, cada espécie, e a simulação que continua.
const STREAM_BURROWS: u64 = 1;
const STREAM_WORLD: u64 = 2;
const STREAM_RABBITS: u64 = 3;
const STREAM_FOXES: u64 = 4;
const STREAM_SIM: u64 = 5;
const STREAM_RESOURCES: u64 = 6;

/// Tentativas para sortear uma célula longe do jogador.
const SPAWN_TRIES: usize = 50;

/// Os números do modelo agregado, medidos na simulação completa.
#[derive(Clone, Debug)]
pub struct AggregateParams {
    /// Nascimentos por coelho adulto por dia, pela reserva (a fração dos
    /// coelhos com energia de `rest_cap` ou mais, ver `AggregateRegion::reserve`):
    /// pares (reserva, taxa), medidos na simulação completa (20 seeds, 900
    /// dias, com e sem choques); entre pontos interpola-se em linha reta. No
    /// fim a tabela sobe até ao máximo possível: uma cria a cada
    /// `repro_cooldown` (5 dias), ou seja 0,2 por dia.
    ///
    /// Com o descanso de noite, a vegetação deixou de chegar para prever as
    /// crias: depois de um choque, com a mesma vegetação, havia o triplo. A
    /// energia média também não chegava (sobe só de 44 para 49). O que as
    /// prevê é quantos coelhos já passaram do descanso e estão a caminho de
    /// `repro_threshold`.
    ///
    /// Com o abrigo das árvores, as tabelas das crias, da reserva e da fome
    /// foram corrigidas pelo que se mediu na completa em equilíbrio. Os
    /// coelhos juntam-se perto das árvores e comem lá mais a erva, por isso,
    /// com a mesma vegetação média, morrem quase o dobro à fome (0,0012 por
    /// dia com 5,0 de vegetação) e têm mais crias (0,0151 por adulto com 10%
    /// de reserva). As crias foram multiplicadas por 1,16, a fome passou 0,93
    /// para a direita e a reserva desceu 0,02.
    pub births: Vec<(f32, f32)>,
    /// A reserva para onde os coelhos caminham, pela erva por coelho (a
    /// vegetação de toda a região a dividir pelos coelhos).
    pub reserve_target: Vec<(f32, f32)>,
    /// A fração da distância a `reserve_target` que a reserva anda por dia.
    /// Medido: depois de um choque passa de 15% para 28% em 4 dias.
    pub reserve_rate: f32,
    /// Mortes por fome por coelho por dia, por vegetação média.
    pub hunger: Vec<(f32, f32)>,
    /// Coelhos comidos por raposa por dia. Era 0,205; com o gasto e o êxito
    /// reais da raposa, e os roedores, mediu-se 0,30 na completa (10 seeds,
    /// 900 dias).
    pub prey_per_fox: f32,
    /// Crias por raposa por dia, pelo número de raposas em cada 24x24 de
    /// região. Medido em cada região de 24x24 de mapas de 72x72, já em
    /// equilíbrio (dias 500 a 1500): as raposas vivem 400 dias, por isso
    /// antes disso ainda nenhuma morreu de velhice e a população ainda está
    /// a subir.
    pub fox_births: Vec<(f32, f32)>,
    /// Vegetação comida por coelho por dia com a vegetação de equilíbrio
    /// (cerca de 4). Com menos vegetação comem menos, com a mesma resposta
    /// funcional do coelho (`half_saturation`). A curva de crescimento diz que,
    /// para a vegetação ficar em 3,85 com 283 coelhos, cada um come 3,46 por
    /// dia; o ajuste à simulação completa deu 3,5.
    pub grazing: f32,
    /// O que a conversão de volta para animais individuais sorteia, com as
    /// distribuições medidas na simulação completa (10 seeds, 5 momentos
    /// depois do aquecimento). Cada tabela é (fração, valor): a fração 0,5
    /// dá a mediana. Vegetação de cada célula a dividir pela média:
    pub biomass_spread: Vec<(f32, f32)>,
    /// Energia de cada coelho e de cada raposa. A dos coelhos é deslocada
    /// para ter a energia média da região (`AggregateRegion::rabbit_energy`).
    pub rabbit_energy: Vec<(f32, f32)>,
    pub fox_energy: Vec<(f32, f32)>,
    /// Fração dos coelhos que está dentro de uma toca. Medida à meia-noite,
    /// como as outras: com o descanso ao meio-dia, a essa hora há poucos
    /// (5%; era 24% com o descanso de noite).
    pub rabbits_in_burrows: f32,
    /// Fração dos coelhos e das raposas que acabou de ter uma cria e ainda
    /// espera para a próxima (`repro_cooldown`).
    pub rabbits_waiting: f32,
    pub foxes_waiting: f32,
    /// Fração das raposas saciadas (`satiety_ticks`).
    pub foxes_satiated: f32,
    /// Fração dos coelhos e das raposas de uma região que passa para a
    /// região vizinha, por cada fronteira e por dia. Medido num mapa de
    /// 48x48 (2x2 regiões), contando só quem, ao fim do dia, está noutra
    /// região do que no início (as idas e voltas na fronteira não contam):
    /// 4,6% dos coelhos e 0,57% das raposas por dia, com 2 fronteiras por
    /// região. As raposas mudam pouco de região porque passam muito tempo
    /// saciadas a descansar e caçam perto.
    pub rabbits_cross: f32,
    pub foxes_cross: f32,
}

impl Default for AggregateParams {
    fn default() -> Self {
        Self {
            births: vec![
                (0.03, 0.0063),
                (0.05, 0.0094),
                (0.07, 0.0116),
                (0.09, 0.0137),
                (0.11, 0.0155),
                (0.13, 0.0168),
                (0.15, 0.0184),
                (0.17, 0.0212),
                (0.19, 0.0251),
                (0.21, 0.0327),
                (0.23, 0.0435),
                (0.25, 0.0644),
                (0.27, 0.0813),
                (0.29, 0.0883),
                (0.31, 0.1052),
                (0.35, 0.1169),
                (0.50, 0.2000),
            ],
            reserve_target: vec![
                (5.0, 0.11),
                (10.0, 0.10),
                (14.0, 0.20),
                (18.0, 0.27),
                (24.0, 0.32),
                (35.0, 0.35),
            ],
            reserve_rate: 0.3,
            hunger: vec![
                (3.430, 0.0300),
                (3.805, 0.0118),
                (4.055, 0.0074),
                (4.305, 0.0048),
                (4.555, 0.0030),
                (4.805, 0.0016),
                (5.055, 0.0010),
                (5.305, 0.0008),
                (5.555, 0.0005),
                (5.805, 0.0003),
                (6.055, 0.0002),
                (6.430, 0.0001),
                (6.930, 0.0000),
            ],
            prey_per_fox: 0.30,
            fox_births: vec![
                // Sozinha: sem rivais, tem uma cria logo que pode (uma a cada
                // `repro_cooldown`, 20 dias). Com companhia, a taxa medida.
                // Com 2 raposas ainda é a medição antiga, num mapa de 24x24:
                // em equilíbrio quase nenhuma região fica com tão poucas.
                (1.0, 0.05),
                (2.0, 0.0039),
                (4.0, 0.0077),
                (6.0, 0.0059),
                (7.0, 0.0053),
                (8.0, 0.0042),
                (9.0, 0.0031),
                (10.0, 0.0029),
                (11.0, 0.0022),
                (12.0, 0.0021),
                (13.0, 0.0016),
                (14.0, 0.0014),
                (15.5, 0.0011),
                (17.5, 0.0008),
            ],
            grazing: 3.3,
            biomass_spread: vec![
                (0.0, 0.25),
                (0.1, 0.44),
                (0.25, 0.57),
                (0.5, 0.86),
                (0.75, 1.32),
                (0.9, 1.79),
                (1.0, 2.6),
            ],
            rabbit_energy: vec![
                (0.0, 1.0),
                (0.1, 23.2),
                (0.25, 29.7),
                (0.5, 38.0),
                (0.75, 52.2),
                (0.9, 61.3),
                (1.0, 91.5),
            ],
            fox_energy: vec![
                (0.0, 17.6),
                (0.1, 56.2),
                (0.25, 69.7),
                (0.5, 79.8),
                (0.75, 89.6),
                (0.9, 92.5),
                (1.0, 100.0),
            ],
            rabbits_in_burrows: 0.05,
            rabbits_waiting: 0.06,
            foxes_waiting: 0.15,
            foxes_satiated: 0.27,
            rabbits_cross: 0.023,
            foxes_cross: 0.0029,
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

/// Numa tabela de (fração, valor) como `rabbit_energy`, a fração dos
/// sorteios que dá menos do que `value` (o inverso de `lookup`).
fn fraction_below(table: &[(f32, f32)], value: f32) -> f32 {
    let (first, last) = (table[0], table[table.len() - 1]);
    if value <= first.1 {
        return first.0;
    }
    if value >= last.1 {
        return last.0;
    }
    let i = table.partition_point(|&(_, v)| v <= value);
    let (f0, v0) = table[i - 1];
    let (f1, v1) = table[i];
    f0 + (f1 - f0) * (value - v0) / (v1 - v0)
}

/// Uma região agregada.
#[derive(Clone, Debug)]
pub struct AggregateRegion {
    /// Número de células da região.
    pub cells: usize,
    /// Vegetação média por célula.
    pub biomass: f32,
    /// A reserva: a fração dos coelhos com energia de `rest_cap` ou mais,
    /// que já não descansam de noite e estão a caminho de ter uma cria. É
    /// ela que diz quantas crias há (ver `AggregateParams::births`).
    pub reserve: f32,
    /// Coelhos por idade em dias (índice 0 = nascidos hoje).
    pub rabbits: Vec<u32>,
    /// Raposas por idade em dias.
    pub foxes: Vec<u32>,
    /// As tocas: são terreno e não mudam quando a região é agregada.
    pub burrows: Vec<(usize, usize)>,
    /// As árvores e as pedras, com o que ainda têm: também são terreno, e
    /// as árvores continuam a crescer enquanto a região está agregada.
    pub resources: Vec<Resource>,
    pub day: u64,
    config: Config,
    params: AggregateParams,
    rng: Rng,
}

impl AggregateRegion {
    pub fn new(config: Config, params: AggregateParams, seed: u64) -> Self {
        let rabbit_days = (config.rabbit.max_age_ticks / TICKS_PER_DAY) as usize + 1;
        let fox_days = (config.fox.max_age_ticks / TICKS_PER_DAY) as usize + 1;
        // Tocas ao acaso, com números à parte para não mudar os da região.
        let mut dig = Rng::derived(&[config.seed, seed, STREAM_BURROWS]);
        let burrows: Vec<(usize, usize)> = (0..config.burrows)
            .map(|_| (dig.range_usize(config.width), dig.range_usize(config.height)))
            .collect();
        let mut dig = Rng::derived(&[config.seed, seed, STREAM_RESOURCES]);
        let resources = config
            .resources
            .place(&mut dig, config.width, config.height, |b| !burrows.contains(&b));
        let reserve = 1.0 - fraction_below(&params.rabbit_energy, config.rabbit.rest_cap);
        Self {
            cells: config.width * config.height,
            biomass: 0.0,
            reserve,
            rabbits: vec![0; rabbit_days],
            foxes: vec![0; fox_days],
            burrows,
            resources,
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
        Self::from_animals(
            sim.config().clone(),
            params,
            seed,
            sim.day(),
            &sim.world,
            &sim.rabbits,
            &sim.foxes,
        )
    }

    /// Como `from_simulation`, mas com os animais e o terreno dados à parte:
    /// a paisagem corta assim uma região de 24x24 da janela completa, que é
    /// maior. `world` tem de ter o tamanho de `config`.
    pub fn from_animals(
        config: Config,
        params: AggregateParams,
        seed: u64,
        day: u64,
        world: &World,
        rabbits: &[Rabbit],
        foxes: &[Fox],
    ) -> Self {
        let mut region = Self::new(config, params, seed);
        region.day = day;
        region.biomass = world.mean_biomass();
        region.burrows = world.burrows().to_vec();
        region.resources = world.resources().to_vec();
        for r in rabbits {
            region.add_rabbit(r.age);
        }
        if !rabbits.is_empty() {
            let cap = region.config.rabbit.rest_cap;
            region.reserve = rabbits.iter().filter(|r| r.energy >= cap).count() as f32 / rabbits.len() as f32;
        }
        for f in foxes {
            region.add_fox(f.age);
        }
        // Se as raposas ainda não chegaram, chegam no mesmo dia que na
        // completa (`step_day`).
        region
    }

    /// Junta um coelho com esta idade (em ticks) à região.
    pub fn add_rabbit(&mut self, age_ticks: u64) {
        let last = self.rabbits.len() - 1;
        self.rabbits[((age_ticks / TICKS_PER_DAY) as usize).min(last)] += 1;
    }

    /// Junta uma raposa com esta idade (em ticks) à região.
    pub fn add_fox(&mut self, age_ticks: u64) {
        let last = self.foxes.len() - 1;
        self.foxes[((age_ticks / TICKS_PER_DAY) as usize).min(last)] += 1;
    }

    pub fn params(&self) -> &AggregateParams {
        &self.params
    }

    pub fn config(&self) -> &Config {
        &self.config
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

    /// Volta a pôr animais individuais na região (GDD 7.9, quando o jogador
    /// se aproxima). Conserva quantos há de cada espécie e de cada idade, a
    /// vegetação média e as tocas. O que a região agregada não guarda (onde
    /// está cada animal, a sua energia, a vegetação de cada célula) é
    /// sorteado com as distribuições medidas na completa. Nenhum animal
    /// aparece a `SPAWN_CLEAR_RADIUS` ou menos do jogador.
    ///
    /// Os sorteios vêm de `Rng::derived` com a seed, a região (`region_id`),
    /// o dia e a espécie: a mesma região expandida no mesmo dia dá sempre o
    /// mesmo resultado, e o número de raposas não mexe onde ficam os coelhos.
    pub fn expand(&self, player: Option<(usize, usize)>, region_id: u64) -> Simulation {
        let mut sim = self.expand_with(region_id, &|cell| far_from(player, cell));
        sim.place_player(player);
        sim
    }

    /// Como `expand`, mas quem diz onde um animal pode aparecer é `clear`.
    /// A paisagem usa isto para regiões ao lado da do jogador, onde ele
    /// está fora do mapa da região mas pode estar perto da fronteira.
    pub fn expand_with(&self, region_id: u64, clear: &dyn Fn((usize, usize)) -> bool) -> Simulation {
        let c = &self.config;
        let p = &self.params;
        let (w, h) = (c.width, c.height);
        let stream = |part: u64| Rng::derived(&[c.seed, region_id, self.day, part]);

        // O terreno: as mesmas tocas e a vegetação com a média certa.
        let mut rng = stream(STREAM_WORLD);
        let mut world = World::new(w, h, c.vegetation.clone(), &mut rng);
        for &(x, y) in &self.burrows {
            world.add_burrow(x, y);
        }
        for r in &self.resources {
            world.add_resource(r.clone());
        }
        let spread: Vec<f32> = (0..w * h).map(|_| lookup(&p.biomass_spread, rng.next_f32())).collect();
        let mut scale = self.biomass / (spread.iter().sum::<f32>() / (w * h) as f32);
        // As células que passariam da capacidade ficam nela e a média desce
        // um pouco; umas voltas a corrigir a escala chegam para a acertar.
        for _ in 0..5 {
            for (i, s) in spread.iter().enumerate() {
                world.set_biomass(i % w, i / w, s * scale);
            }
            let mean = world.mean_biomass();
            if mean <= 0.0 || (mean - self.biomass).abs() < 1e-4 {
                break;
            }
            scale *= self.biomass / mean;
        }

        let mut rng = stream(STREAM_RABBITS);
        let hidden: Vec<(usize, usize)> =
            world.burrows().iter().copied().filter(|&b| clear(b)).collect();
        let mut rabbits = Vec::with_capacity(self.rabbit_count() as usize);
        for (days, &n) in self.rabbits.iter().enumerate() {
            for _ in 0..n {
                let (x, y) = if !hidden.is_empty() && rng.next_f32() < p.rabbits_in_burrows {
                    hidden[rng.range_usize(hidden.len())]
                } else {
                    far_cell(&mut rng, w, h, clear)
                };
                rabbits.push(self.new_rabbit(&mut rng, days, (x, y)));
            }
        }

        // As raposas ficam umas perto das outras, como na completa, onde
        // todas sobem os mesmos gradientes de cheiro e quase nenhuma está sem
        // rival a `territory_radius` ou menos. Espalhadas ao acaso, as que
        // ficavam sozinhas tinham crias logo (o travão de densidade só vê
        // rivais perto): 44 crias nos primeiros 25 dias, contra 10.
        let mut rng = stream(STREAM_FOXES);
        let mut foxes: Vec<Fox> = Vec::with_capacity(self.fox_count() as usize);
        for (days, &n) in self.foxes.iter().enumerate() {
            for _ in 0..n {
                let (x, y) = if foxes.is_empty() {
                    far_cell(&mut rng, w, h, clear)
                } else {
                    let anchor = &foxes[rng.range_usize(foxes.len())];
                    near_cell(&mut rng, w, h, (anchor.x, anchor.y), c.fox.territory_radius, clear)
                };
                foxes.push(self.new_fox(&mut rng, days, (x, y)));
            }
        }

        let tick = self.day * TICKS_PER_DAY;
        Simulation::from_parts(c.clone(), stream(STREAM_SIM), world, rabbits, foxes, tick)
    }

    /// Um coelho individual com `days` dias de idade, nesta célula, com a
    /// energia e a espera para ter crias sorteadas como na completa.
    pub fn new_rabbit(&self, rng: &mut Rng, days: usize, (x, y): (usize, usize)) -> Rabbit {
        let (c, p) = (&self.config, &self.params);
        // Sorteia de cima ou de baixo de `rest_cap` para respeitar a reserva.
        let below = fraction_below(&p.rabbit_energy, c.rabbit.rest_cap);
        let u = rng.next_f32();
        let f = if rng.next_f32() < self.reserve { below + (1.0 - below) * u } else { below * u };
        let mut r = Rabbit::newborn(x, y, lookup(&p.rabbit_energy, f));
        r.age = some_hour_of(rng, days).min(c.rabbit.max_age_ticks);
        if rng.next_f32() < p.rabbits_waiting {
            r.cooldown = 1 + rng.range_usize(c.rabbit.repro_cooldown as usize) as u64;
        }
        r
    }

    /// Uma raposa individual, como `new_rabbit`.
    pub fn new_fox(&self, rng: &mut Rng, days: usize, (x, y): (usize, usize)) -> Fox {
        let (c, p) = (&self.config, &self.params);
        let mut f = Fox::newborn(x, y, lookup(&p.fox_energy, rng.next_f32()));
        f.age = some_hour_of(rng, days).min(c.fox.max_age_ticks);
        if rng.next_f32() < p.foxes_waiting {
            f.cooldown = 1 + rng.range_usize(c.fox.repro_cooldown as usize) as u64;
        }
        if rng.next_f32() < p.foxes_satiated {
            f.satiety = 1 + rng.range_usize(c.fox.satiety_ticks as usize) as u64;
        }
        f
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
        self.config.resources.regrow(&mut self.resources, 1.0);
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
        let births = self.poisson(rabbit_adults * lookup(&p.births, self.reserve));
        if r > 0.0 {
            let grass_per_rabbit = b * self.cells as f32 / r;
            self.reserve += p.reserve_rate * (lookup(&p.reserve_target, grass_per_rabbit) - self.reserve);
        }
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

/// Uma célula ao acaso onde `clear` deixa aparecer um animal. Num mapa
/// pequeno demais para isso, ao fim de algumas tentativas fica a última.
fn far_cell(rng: &mut Rng, w: usize, h: usize, clear: &dyn Fn((usize, usize)) -> bool) -> (usize, usize) {
    let mut cell = (rng.range_usize(w), rng.range_usize(h));
    for _ in 0..SPAWN_TRIES {
        if clear(cell) {
            break;
        }
        cell = (rng.range_usize(w), rng.range_usize(h));
    }
    cell
}

/// Uma célula ao acaso a `radius` ou menos de `anchor` onde `clear` deixa
/// aparecer um animal. Se não houver nenhuma, uma em qualquer sítio.
fn near_cell(
    rng: &mut Rng,
    w: usize,
    h: usize,
    anchor: (usize, usize),
    radius: usize,
    clear: &dyn Fn((usize, usize)) -> bool,
) -> (usize, usize) {
    let (x0, y0) = (anchor.0.saturating_sub(radius), anchor.1.saturating_sub(radius));
    let (x1, y1) = ((anchor.0 + radius).min(w - 1), (anchor.1 + radius).min(h - 1));
    for _ in 0..SPAWN_TRIES {
        let cell = (x0 + rng.range_usize(x1 - x0 + 1), y0 + rng.range_usize(y1 - y0 + 1));
        if distance(cell, anchor) <= radius && clear(cell) {
            return cell;
        }
    }
    far_cell(rng, w, h, clear)
}

/// Uma idade em ticks dentro do dia `days` de vida: a região só sabe o dia.
fn some_hour_of(rng: &mut Rng, days: usize) -> u64 {
    days as u64 * TICKS_PER_DAY + rng.range_usize(TICKS_PER_DAY as usize) as u64
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
    use crate::sim::SPAWN_CLEAR_RADIUS;

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

    /// Média de coelhos, raposas e vegetação nos dias `dias` depois do fim
    /// do aquecimento, em `seeds` seeds, nas duas simulações, num mapa de
    /// `lado` x `lado` com a densidade de animais e tocas do de 24x24.
    fn medias(seeds: u64, lado: usize, dias: std::ops::Range<u64>) -> ([f32; 3], [f32; 3]) {
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
            for dia in 0..dias.end {
                sim.run_days(1);
                r.step_day();
                if dia >= dias.start {
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
    /// pela outra quando o jogador se afasta. O equilíbrio das raposas só
    /// chega depois de as primeiras morrerem de velhice (400 dias), por isso
    /// compara-se dos 500 aos 900 dias.
    #[test]
    fn a_agregada_tem_o_mesmo_equilibrio_que_a_completa() {
        let (c, a) = medias(10, 24, 500..900);
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
        let (c, a) = medias(4, 48, 500..900);
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

    /// GDD 18.6 e 7.9: de agregada para animais individuais, a população
    /// conserva-se, idade a idade, e a vegetação média e as tocas também.
    #[test]
    fn expandir_conserva_a_populacao() {
        for seed in 1..=5 {
            let sim = aquecida(seed);
            let mut r = AggregateRegion::from_simulation(&sim, AggregateParams::default(), seed);
            for _ in 0..20 {
                r.step_day();
            }
            let expandida = r.expand(Some((12, 12)), 3);
            let de_novo = AggregateRegion::from_simulation(&expandida, AggregateParams::default(), seed);
            assert_eq!(de_novo.rabbits, r.rabbits, "seed {seed}: coelhos por idade");
            assert_eq!(de_novo.foxes, r.foxes, "seed {seed}: raposas por idade");
            assert!((de_novo.biomass - r.biomass).abs() < 1e-3, "seed {seed}: vegetação");
            assert_eq!(expandida.world.burrows(), sim.world.burrows());
            assert_eq!(expandida.day(), r.day);
        }
    }

    /// Quando o jogador se aproxima, nenhum animal aparece do nada perto dele,
    /// esteja ele no meio, na margem ou num canto do mapa.
    #[test]
    fn nada_aparece_perto_do_jogador() {
        for seed in 1..=5 {
            let r = AggregateRegion::from_simulation(&aquecida(seed), AggregateParams::default(), seed);
            for jogador in [(12, 12), (0, 0), (23, 10), (5, 20)] {
                let sim = r.expand(Some(jogador), 0);
                let posicoes = sim.rabbits.iter().map(|a| (a.x, a.y)).chain(sim.foxes.iter().map(|f| (f.x, f.y)));
                for p in posicoes {
                    assert!(distance(p, jogador) > SPAWN_CLEAR_RADIUS, "seed {seed}: animal em {p:?}, jogador em {jogador:?}");
                }
            }
        }
    }

    /// A mesma região expandida no mesmo dia dá sempre os mesmos animais;
    /// outra região com os mesmos números dá outros.
    #[test]
    fn expandir_e_deterministico() {
        let r = AggregateRegion::from_simulation(&aquecida(4), AggregateParams::default(), 4);
        let sitios = |sim: &Simulation| sim.rabbits.iter().map(|a| (a.x, a.y, a.age)).collect::<Vec<_>>();
        assert_eq!(sitios(&r.expand(None, 1)), sitios(&r.expand(None, 1)));
        assert_ne!(sitios(&r.expand(None, 1)), sitios(&r.expand(None, 2)));
    }

    /// Ida e volta: uma região agregada e logo expandida continua como a
    /// completa continuaria, sem a conversão causar fomes ou explosões de
    /// crias. As raposas espalhadas ao acaso tinham crias de repente, porque
    /// as sozinhas não têm travão: é o que este teste apanha.
    #[test]
    fn ida_e_volta_mantem_o_equilibrio() {
        let jogador = Some((12, 12));
        let (mut completa, mut expandida) = ([0.0f32; 2], [0.0f32; 2]);
        let mut crias = [0; 2];
        for seed in 1..=10 {
            let mut sim = aquecida(seed);
            let mut volta = AggregateRegion::from_simulation(&sim, AggregateParams::default(), seed).expand(jogador, 0);
            sim.place_player(jogador);
            let antes = sim.stats.len();
            for dia in 0..100 {
                sim.run_days(1);
                volta.run_days(1);
                if dia >= 10 {
                    completa[0] += sim.rabbits.len() as f32;
                    completa[1] += sim.foxes.len() as f32;
                    expandida[0] += volta.rabbits.len() as f32;
                    expandida[1] += volta.foxes.len() as f32;
                }
            }
            crias[0] += sim.stats[antes..antes + 25].iter().map(|d| d.fox_births).sum::<usize>();
            crias[1] += volta.stats[..25].iter().map(|d| d.fox_births).sum::<usize>();
            assert_eq!(volta.migrations_rabbits + volta.migrations_foxes, 0, "seed {seed}: migração");
        }
        let perto = |x: f32, y: f32, tol: f32| (x - y).abs() <= tol * x;
        assert!(perto(completa[0], expandida[0], 0.08), "coelhos: completa {}, expandida {}", completa[0], expandida[0]);
        assert!(perto(completa[1], expandida[1], 0.25), "raposas: completa {}, expandida {}", completa[1], expandida[1]);
        assert!(crias[1] <= 2 * crias[0].max(5), "crias de raposa nos primeiros 25 dias: completa {}, expandida {}", crias[0], crias[1]);
    }
}
