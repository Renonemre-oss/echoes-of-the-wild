//! A simulação: junta o mundo, os coelhos e as raposas e avança no tempo.

use crate::daylight::{self, DaylightParams};
use crate::fox::{Fox, FoxParams};
use crate::rabbit::{Rabbit, RabbitParams};
use crate::resources::{Resource, ResourceParams};
use crate::rng::Rng;
use crate::scent::{Scent, ScentParams};
use crate::spatial::Grid;
use crate::world::{distance, VegetationParams, World};

/// 1 tick = 1 hora de jogo. 24 ticks = 1 dia.
pub const TICKS_PER_DAY: u64 = 24;

/// O lado mínimo (em células) de uma região simulada com coelhos e raposas.
/// Na varredura, 20x20 ainda aguenta 1000 dias, mas com 18x18 ou menos há tão
/// poucas raposas que desaparecem por azar; 24 deixa margem (ver DIARIO.md). Os chunks do
/// mundo quase infinito podem ser mais pequenos, mas a simulação agregada
/// (GDD 7.9) tem de juntar chunks até chegar a regiões deste tamanho.
pub const MIN_REGION_SIZE: usize = 24;

/// Com coelhos × raposas abaixo disto, procurar na lista inteira é mais rápido
/// do que usar as grelhas espaciais. Num mapa de 24x24 (288 × 4) a grelha
/// deixava o tick duas vezes mais lento, e num de 48x48 (1245 × 16) ainda 30%;
/// a partir de 96x96 (5000 × 63) já ganha. Os dois caminhos dão exatamente o
/// mesmo resultado.
const GRID_MIN_PAIRS: usize = 100_000;

/// Os animais que entram no mapa (migração, chegada das raposas, conversão
/// de uma região agregada) aparecem a mais do que isto do jogador. É o dobro
/// do medo das raposas (`human_fear`) e mais do que a vista do coelho, por
/// isso nada surge de repente ao lado dele.
pub const SPAWN_CLEAR_RADIUS: usize = 8;

/// Tentativas para encontrar uma célula longe do jogador.
const SPAWN_TRIES: usize = 50;

/// A sequência de números ao acaso das árvores e das pedras (ver `Rng::derived`).
const STREAM_RESOURCES: u64 = 21;

/// A célula fica a mais de `SPAWN_CLEAR_RADIUS` do jogador (ou não há jogador).
pub fn far_from(player: Option<(usize, usize)>, cell: (usize, usize)) -> bool {
    player.is_none_or(|p| distance(p, cell) > SPAWN_CLEAR_RADIUS)
}

/// Tamanhos de população que merecem ser celebrados.
const POPULATION_MILESTONES: [usize; 4] = [50, 100, 200, 300];

/// Migração de segurança (GDD 7.8, mecanismo 6): se uma espécie ficar abaixo de
/// um mínimo durante vários dias, chega um indivíduo pela margem do mapa.
/// Um mínimo de 0 desliga a migração dessa espécie.
#[derive(Clone, Debug)]
pub struct MigrationParams {
    pub min_rabbits: usize,
    pub min_foxes: usize,
    pub days_below: u64,
}

impl Default for MigrationParams {
    fn default() -> Self {
        Self {
            min_rabbits: 5,
            min_foxes: 1,
            days_below: 3,
        }
    }
}

impl MigrationParams {
    pub fn off() -> Self {
        Self {
            min_rabbits: 0,
            min_foxes: 0,
            days_below: 0,
        }
    }
}

/// O teste da cabana (GDD 18.4): todos os dias o jogador deixa comida junto à
/// cabana, e contamos quantas vezes cada espécie entra na zona à volta.
#[derive(Clone, Debug)]
pub struct CabinParams {
    pub x: usize,
    pub y: usize,
    /// A zona da cabana: células a esta distância ou menos.
    pub radius: usize,
    /// Restos de comida que o jogador deixa por dia na célula da cabana.
    /// 0 é a execução de controlo: a mesma cabana, sem comida.
    pub food_per_day: f32,
}

impl CabinParams {
    /// A cabana fica numa clareira: nenhuma árvore ou pedra abriga a zona
    /// dela. Uma árvore ao lado da cabana protegia os coelhos que vinham
    /// pela comida, e as raposas deixavam de caçar lá.
    pub fn in_clearing(&self, cell: (usize, usize)) -> bool {
        distance(cell, (self.x, self.y)) <= self.radius + 2 * crate::world::TREE_COVER
    }

    /// Uma cabana no centro de um mapa com estas dimensões.
    pub fn centered(width: usize, height: usize, food_per_day: f32) -> Self {
        Self {
            x: width / 2,
            y: height / 2,
            radius: 2,
            food_per_day,
        }
    }

    fn contains(&self, x: usize, y: usize) -> bool {
        distance((x, y), (self.x, self.y)) <= self.radius
    }
}

/// Intervalo máximo entre o primeiro e o último acontecimento de uma cadeia.
const CHAIN_WINDOW_TICKS: u64 = 2 * TICKS_PER_DAY;

/// O registo do que acontece junto à cabana (GDD 18.6 e 20.12), em frases
/// legíveis, e as cadeias de acontecimentos que ninguém escreveu: um coelho
/// come os restos e, pouco depois, uma raposa caça um coelho junto à cabana.
/// A raposa pode ter vindo atrás ou já lá estar (porque a comida também a
/// atrai): as duas contam, porque em ambas a comida trouxe a presa.
#[derive(Clone, Debug, Default)]
pub struct CabinLog {
    pub lines: Vec<String>,
    /// Quantas cadeias coelho → raposa → caça aconteceram.
    pub chains: usize,
    /// Caçadas na zona da cabana, com ou sem cadeia.
    pub hunts: usize,
    /// Tick em que um coelho comeu os restos (início de uma possível cadeia).
    rabbit_ate: Option<u64>,
    /// Tick em que uma raposa chegou à zona depois disso.
    fox_came: Option<u64>,
    /// Último dia em que se registou uma raposa a comer restos (para não repetir).
    fox_ate_day: Option<u64>,
}

impl CabinLog {
    fn say(&mut self, tick: u64, text: &str) {
        self.lines.push(format!(
            "Dia {}, {:02}h: {text}",
            tick / TICKS_PER_DAY,
            tick % TICKS_PER_DAY
        ));
    }

    fn still_open(start: Option<u64>, now: u64) -> bool {
        start.is_some_and(|t| now - t <= CHAIN_WINDOW_TICKS)
    }

    fn rabbit_ate_scraps(&mut self, tick: u64) {
        // Só se regista o início de uma sequência nova, senão eram centenas de linhas.
        if !Self::still_open(self.rabbit_ate, tick) {
            self.rabbit_ate = Some(tick);
            self.fox_came = None;
            self.say(tick, "um coelho comeu os restos junto à cabana.");
        }
    }

    fn fox_ate_scraps(&mut self, tick: u64) {
        let day = tick / TICKS_PER_DAY;
        if self.fox_ate_day != Some(day) {
            self.fox_ate_day = Some(day);
            self.say(tick, "uma raposa comeu os restos junto à cabana.");
        }
    }

    fn fox_arrived(&mut self, tick: u64, followed_food: bool) {
        if followed_food {
            self.say(tick, "uma raposa seguiu o cheiro dos restos até à cabana.");
        } else {
            self.say(tick, "uma raposa chegou à zona da cabana.");
        }
        if Self::still_open(self.rabbit_ate, tick) && self.fox_came.is_none() {
            self.fox_came = Some(tick);
        }
    }

    fn fox_hunted(&mut self, tick: u64) {
        self.hunts += 1;
        self.say(tick, "uma raposa caçou um coelho junto à cabana.");
        if Self::still_open(self.rabbit_ate, tick) {
            self.chains += 1;
            let how = if self.fox_came.is_some() {
                "→ cadeia: o coelho veio pela comida, a raposa veio atrás, e houve uma caçada."
            } else {
                "→ cadeia: o coelho veio pela comida, e a raposa já estava à espera."
            };
            self.say(tick, how);
            self.rabbit_ate = None;
            self.fox_came = None;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub seed: u64,
    pub width: usize,
    pub height: usize,
    pub initial_rabbits: usize,
    pub initial_foxes: usize,
    /// Dia em que as raposas iniciais chegam. Antes disso só há coelhos.
    /// Com 20 coelhos e raposas desde o dia 0, 4 ou mais raposas comiam-nos
    /// antes de se reproduzirem; com 20 dias de avanço, 2 a 8 raposas são
    /// estáveis (ver DIARIO.md).
    pub foxes_arrive_day: u64,
    /// Dias simulados antes de o jogador chegar (`Simulation::warmed_up`),
    /// para o mundo já estar em equilíbrio quando ele aparece.
    pub warmup_days: u64,
    /// Número de tocas de coelho, espalhadas ao acaso.
    pub burrows: usize,
    pub vegetation: VegetationParams,
    pub rabbit: RabbitParams,
    pub fox: FoxParams,
    pub migration: MigrationParams,
    pub scent: ScentParams,
    pub daylight: DaylightParams,
    pub resources: ResourceParams,
    pub cabin: Option<CabinParams>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 42,
            width: 24,
            height: 24,
            initial_rabbits: 20,
            initial_foxes: 2,
            foxes_arrive_day: 30,
            warmup_days: 60,
            burrows: 30,
            vegetation: VegetationParams::default(),
            rabbit: RabbitParams::default(),
            fox: FoxParams::default(),
            migration: MigrationParams::default(),
            scent: ScentParams::default(),
            daylight: DaylightParams::default(),
            resources: ResourceParams::default(),
            cabin: None,
        }
    }
}

impl Config {
    /// O mundo só com coelhos e vegetação, como antes da raposa.
    pub fn without_foxes() -> Self {
        Self {
            initial_foxes: 0,
            migration: MigrationParams::off(),
            ..Self::default()
        }
    }
}

/// Resumo de um dia de simulação (uma linha do relatório CSV).
#[derive(Clone, Debug)]
pub struct DayStats {
    pub day: u64,
    pub rabbits: usize,
    pub births: usize,
    pub deaths_hunger: usize,
    pub deaths_age: usize,
    /// Coelhos caçados por raposas.
    pub deaths_predation: usize,
    pub foxes: usize,
    pub fox_births: usize,
    pub fox_deaths_hunger: usize,
    pub fox_deaths_age: usize,
    /// Animais (de qualquer espécie) que chegaram por migração neste dia.
    pub migrations: usize,
    pub mean_biomass: f32,
    /// Entradas na zona da cabana (0 se não houver cabana).
    pub cabin_visits_rabbits: usize,
    pub cabin_visits_foxes: usize,
    /// Horas passadas na zona da cabana, somadas por todos os animais da espécie.
    pub cabin_hours_rabbits: usize,
    pub cabin_hours_foxes: usize,
}

/// Contadores que se vão somando ao longo de um dia e se põem a zero no fim.
#[derive(Clone, Debug, Default)]
struct DayCounters {
    births: usize,
    deaths_hunger: usize,
    deaths_age: usize,
    deaths_predation: usize,
    fox_births: usize,
    fox_deaths_hunger: usize,
    fox_deaths_age: usize,
    migrations: usize,
    cabin_visits_rabbits: usize,
    cabin_visits_foxes: usize,
    cabin_hours_rabbits: usize,
    cabin_hours_foxes: usize,
}

#[derive(Default)]
struct Scratch {
    fox_grid: Grid,
    rabbit_grid: Grid,
    fox_positions: Vec<(usize, usize)>,
    nearby: Vec<usize>,
    threats: Vec<(usize, usize)>,
}

pub struct Simulation {
    config: Config,
    rng: Rng,
    pub world: World,
    pub rabbits: Vec<Rabbit>,
    pub foxes: Vec<Fox>,
    /// Onde está o jogador, se estiver no mapa. Os coelhos e as raposas
    /// afastam-se dele. Sem jogador, a simulação é igual à de antes.
    pub player: Option<(usize, usize)>,
    tick: u64,
    /// Grelhas espaciais e listas auxiliares, refeitas em cada tick e
    /// guardadas aqui só para não alocar memória de novo.
    scratch: Scratch,
    /// A partir de quantos pares coelho × raposa se usam as grelhas
    /// (`GRID_MIN_PAIRS`; os testes mudam-no para comparar os dois caminhos).
    grid_min_pairs: usize,
    today: DayCounters,
    pub stats: Vec<DayStats>,
    /// Acontecimentos importantes, em texto legível (GDD 20.12).
    pub events: Vec<String>,
    /// Quantas vezes a migração de segurança trouxe cada espécie (GDD 18.6).
    pub migrations_rabbits: usize,
    pub migrations_foxes: usize,
    /// O que aconteceu junto à cabana (vazio se não houver cabana).
    pub cabin_log: CabinLog,
    rabbit_low_days: u64,
    fox_low_days: u64,
    foxes_arrived: bool,
    first_scraps_logged: bool,
    first_birth_logged: bool,
    first_hunt_logged: bool,
    first_fox_birth_logged: bool,
    next_milestone: usize,
    extinction_logged: bool,
    foxes_present: bool,
}

impl Simulation {
    pub fn new(config: Config) -> Self {
        let mut rng = Rng::new(config.seed);
        let mut world = World::new(config.width, config.height, config.vegetation.clone(), &mut rng);
        let mut rabbits = Vec::with_capacity(config.initial_rabbits);
        for _ in 0..config.initial_rabbits {
            let x = rng.range_usize(config.width);
            let y = rng.range_usize(config.height);
            let energy = config.rabbit.energy_start * rng.range_f32(0.7, 1.0);
            let mut rabbit = Rabbit::newborn(x, y, energy);
            // Os coelhos iniciais já são adultos, para a população arrancar.
            rabbit.age = config.rabbit.maturity_ticks;
            rabbits.push(rabbit);
        }
        // As tocas vêm depois dos coelhos, e as raposas só chegam mais tarde
        // (`foxes_arrive_day`): assim o mundo, os coelhos e as tocas de uma seed
        // são os mesmos com ou sem raposas.
        for _ in 0..config.burrows {
            let x = rng.range_usize(config.width);
            let y = rng.range_usize(config.height);
            world.add_burrow(x, y);
        }
        // As árvores e as pedras têm números à parte: assim o resto do mundo
        // de uma seed é o mesmo que antes de existirem.
        let mut dig = Rng::derived(&[config.seed, STREAM_RESOURCES]);
        // Sorteiam-se como sem cabana, e só depois se tiram os da clareira:
        // assim o resto do terreno é o mesmo com e sem cabana.
        let placed = config.resources.place(&mut dig, config.width, config.height, |(x, y)| !world.is_burrow(x, y));
        let clearing = |r: &Resource| config.cabin.as_ref().is_some_and(|c| c.in_clearing((r.x, r.y)));
        for r in placed.into_iter().filter(|r| !clearing(r)) {
            world.add_resource(r);
        }
        let mut sim = Self::assemble(config, rng, world, rabbits, Vec::new(), 0);
        sim.leave_cabin_food();
        if sim.config.foxes_arrive_day == 0 {
            sim.foxes_arrive();
        }
        sim
    }

    /// Junta as peças numa simulação, com os contadores e o registo a zero.
    fn assemble(
        config: Config,
        rng: Rng,
        world: World,
        rabbits: Vec<Rabbit>,
        foxes: Vec<Fox>,
        tick: u64,
    ) -> Self {
        Self {
            config,
            rng,
            world,
            rabbits,
            foxes,
            player: None,
            tick,
            scratch: Scratch::default(),
            grid_min_pairs: GRID_MIN_PAIRS,
            today: DayCounters::default(),
            stats: Vec::new(),
            events: Vec::new(),
            migrations_rabbits: 0,
            migrations_foxes: 0,
            cabin_log: CabinLog::default(),
            rabbit_low_days: 0,
            fox_low_days: 0,
            foxes_arrived: false,
            first_scraps_logged: false,
            first_birth_logged: false,
            first_hunt_logged: false,
            first_fox_birth_logged: false,
            next_milestone: 0,
            extinction_logged: false,
            foxes_present: false,
        }
    }

    /// Uma simulação a meio da vida do mundo, feita a partir de peças (a
    /// conversão de uma região agregada, GDD 7.9). Os primeiros
    /// acontecimentos ("nasceu o primeiro coelho") já aconteceram antes, por
    /// isso não voltam a ser anunciados.
    pub fn from_parts(
        config: Config,
        rng: Rng,
        world: World,
        rabbits: Vec<Rabbit>,
        foxes: Vec<Fox>,
        tick: u64,
    ) -> Self {
        let mut sim = Self::assemble(config, rng, world, rabbits, foxes, tick);
        sim.foxes_arrived = sim.day() >= sim.config.foxes_arrive_day;
        sim.foxes_present = !sim.foxes.is_empty();
        sim.first_scraps_logged = true;
        sim.first_birth_logged = true;
        sim.first_hunt_logged = true;
        sim.first_fox_birth_logged = true;
        let n = sim.rabbits.len();
        sim.next_milestone = POPULATION_MILESTONES.iter().filter(|&&m| n >= m).count();
        sim.leave_cabin_food();
        sim
    }

    /// Um mundo já aquecido: corre `warmup_days` dias, para o jogador chegar a
    /// um ecossistema em equilíbrio e não ao arranque artificial do dia 0.
    /// O histórico do aquecimento fica em `stats` e `events`, para se poder
    /// ver se o arranque precisou de migrações.
    pub fn warmed_up(config: Config) -> Self {
        let days = config.warmup_days;
        let mut sim = Self::new(config);
        sim.run_days(days);
        sim
    }

    /// Põe o jogador numa célula (ou tira-o do mapa, com `None`).
    pub fn place_player(&mut self, cell: Option<(usize, usize)>) {
        self.player = cell.map(|(x, y)| (x.min(self.world.width - 1), y.min(self.world.height - 1)));
    }

    /// Anda uma célula na direção (dx, dy), sem sair do mapa. Não gasta
    /// tempo de jogo: o jogador mexe-se em tempo real, mesmo em pausa.
    pub fn move_player(&mut self, dx: i32, dy: i32) {
        if let Some((x, y)) = self.player {
            let nx = (x as i32 + dx).clamp(0, self.world.width as i32 - 1) as usize;
            let ny = (y as i32 + dy).clamp(0, self.world.height as i32 - 1) as usize;
            self.player = Some((nx, ny));
        }
    }

    /// As raposas iniciais chegam, em células ao acaso.
    fn foxes_arrive(&mut self) {
        self.foxes_arrived = true;
        for _ in 0..self.config.initial_foxes {
            let (x, y) = self.spawn_cell(|rng, w, h| (rng.range_usize(w), rng.range_usize(h)));
            let mut fox = Fox::newborn(x, y, self.config.fox.energy_start);
            fox.age = self.config.fox.maturity_ticks;
            self.foxes.push(fox);
        }
        if !self.foxes.is_empty() {
            self.foxes_present = true;
            self.events
                .push(format!("Dia {}: chegaram as raposas. 🦊", self.day()));
        }
    }

    /// As fontes depositam cheiro na sua célula e depois o cheiro espalha-se.
    fn update_scent(&mut self) {
        let p = &self.config.scent;
        if let Some(c) = &self.config.cabin {
            let scraps = self.world.scraps_at(c.x, c.y);
            if scraps > 0.0 {
                self.world.scent.deposit(Scent::Food, c.x, c.y, scraps * p.food_per_scrap);
            }
        }
        for r in &self.rabbits {
            if !self.world.is_burrow(r.x, r.y) {
                self.world.scent.deposit(Scent::Prey, r.x, r.y, p.prey_per_rabbit);
            }
        }
        if let Some((x, y)) = self.player {
            self.world.scent.deposit(Scent::Human, x, y, p.human_per_tick);
        }
        self.world.scent.spread(p);
    }

    /// O jogador repõe a comida junto à cabana (uma vez por dia).
    fn leave_cabin_food(&mut self) {
        if let Some(c) = &self.config.cabin {
            let missing = c.food_per_day - self.world.scraps_at(c.x, c.y);
            if missing > 0.0 {
                self.world.add_scraps(c.x, c.y, missing);
            }
        }
    }

    pub fn day(&self) -> u64 {
        self.tick / TICKS_PER_DAY
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// A luz agora: 1 de dia, 0 de noite (ver `DaylightParams`).
    pub fn light(&self) -> f32 {
        self.config.daylight.light(self.tick % TICKS_PER_DAY)
    }

    /// Avança um tick (uma hora de jogo).
    pub fn step(&mut self) {
        self.tick += 1;
        self.world.grow();
        self.world.regrow_resources(&self.config.resources, 1.0 / TICKS_PER_DAY as f32);
        self.update_scent();
        // Com a luz desta hora, cada animal vê até onde lhe deixa a escuridão.
        let light = self.light();
        let mut fox_params = self.config.fox.clone();
        fox_params.vision = daylight::vision(fox_params.vision, fox_params.night_vision, light);
        fox_params.hunt_below += (fox_params.day_hunt_below - fox_params.hunt_below) * light;
        let mut rabbit_params = self.config.rabbit.clone();
        rabbit_params.vision = daylight::vision(rabbit_params.vision, rabbit_params.night_vision, light);
        if light < 0.5 {
            rabbit_params.rest_above = rabbit_params.night_rest_above;
        }

        // Primeiro as raposas: caçam e marcam os coelhos apanhados como `eaten`.
        // As posições no início do tick servem para contar rivais perto (travão de densidade).
        // As grelhas espaciais deixam cada animal procurar só à sua volta.
        let (w, h) = (self.world.width, self.world.height);
        // Tira as grelhas da simulação durante o tick (e devolve-as no fim),
        // para poderem ser usadas ao mesmo tempo que o resto da simulação.
        let mut scratch = std::mem::take(&mut self.scratch);
        let Scratch {
            fox_grid,
            rabbit_grid,
            fox_positions,
            nearby,
            threats,
        } = &mut scratch;
        fox_positions.clear();
        fox_positions.extend(self.foxes.iter().map(|f| (f.x, f.y)));
        let fox_starts = &*fox_positions;
        let use_grids = self.foxes.len() * self.rabbits.len() >= self.grid_min_pairs;
        let (use_fox_grid, use_rabbit_grid) = (use_grids, use_grids);
        if use_fox_grid {
            fox_grid.rebuild(w, h, fox_starts.iter().copied());
        }
        if use_rabbit_grid {
            rabbit_grid.rebuild(w, h, self.rabbits.iter().map(|r| (r.x, r.y)));
        }
        let radius = self.config.fox.territory_radius;
        let mut fox_newborns = Vec::new();
        let mut hunts = 0;
        let cabin = self.config.cabin.clone();
        let in_cabin = |x, y| cabin.as_ref().is_some_and(|c| c.contains(x, y));
        let scraps_before = cabin.as_ref().map_or(0.0, |c| self.world.scraps_at(c.x, c.y));
        for (i, fox) in self.foxes.iter_mut().enumerate() {
            let is_rival = |j: usize| j != i && distance(fox_starts[j], fox_starts[i]) <= radius;
            let rivals = if use_fox_grid {
                fox_grid.near(fox_starts[i], radius, nearby);
                nearby.iter().filter(|&&j| is_rival(j)).count()
            } else {
                (0..fox_starts.len()).filter(|&j| is_rival(j)).count()
            };
            let candidates = if use_rabbit_grid {
                rabbit_grid.near((fox.x, fox.y), fox_params.vision, nearby);
                Some(nearby.as_slice())
            } else {
                None
            };
            let was_in = in_cabin(fox.x, fox.y);
            let scraps = cabin.as_ref().map_or(0.0, |c| self.world.scraps_at(c.x, c.y));
            let result = fox.act_near(
                &mut self.rabbits,
                candidates,
                &mut self.world,
                &mut self.rng,
                &fox_params,
                rivals,
                self.player,
            );
            if result.caught {
                hunts += 1;
            }
            if in_cabin(fox.x, fox.y) {
                self.today.cabin_hours_foxes += 1;
                if !was_in {
                    self.today.cabin_visits_foxes += 1;
                    self.cabin_log.fox_arrived(self.tick, result.followed_food);
                }
                if result.caught {
                    // Depois de caçar, a raposa fica na célula do coelho.
                    self.cabin_log.fox_hunted(self.tick);
                }
            }
            if let Some(c) = &cabin {
                if self.world.scraps_at(c.x, c.y) < scraps {
                    self.cabin_log.fox_ate_scraps(self.tick);
                }
            }
            fox_newborns.extend(result.baby);
        }
        if let Some(c) = &cabin {
            if self.world.scraps_at(c.x, c.y) < scraps_before && !self.first_scraps_logged {
                self.first_scraps_logged = true;
                self.events.push(format!(
                    "Dia {}: uma raposa comeu os restos junto à cabana.",
                    self.day()
                ));
            }
        }
        if hunts > 0 && !self.first_hunt_logged {
            self.first_hunt_logged = true;
            self.events
                .push(format!("Dia {}: uma raposa caçou o primeiro coelho. 🦊", self.day()));
        }

        // Depois os coelhos, pela ordem em que estão na lista (determinístico).
        // Veem onde as raposas estão agora e fogem se alguma estiver perto.
        // Cada coelho só recebe as raposas perto dele (da grelha, pela ordem da
        // lista) e o jogador, que também conta como ameaça.
        fox_positions.clear();
        fox_positions.extend(self.foxes.iter().map(|f| (f.x, f.y)));
        let use_fox_grid = self.foxes.len() * self.rabbits.len() >= self.grid_min_pairs;
        if use_fox_grid {
            fox_grid.rebuild(w, h, fox_positions.iter().copied());
        } else {
            // Poucas raposas: todos os coelhos recebem a lista inteira.
            threats.clear();
            threats.extend(fox_positions.iter().copied().chain(self.player));
        }
        let vision = rabbit_params.vision;
        let mut newborns = Vec::new();
        for rabbit in self.rabbits.iter_mut() {
            if rabbit.eaten {
                continue;
            }
            let was_in = in_cabin(rabbit.x, rabbit.y);
            let scraps = cabin.as_ref().map_or(0.0, |c| self.world.scraps_at(c.x, c.y));
            if use_fox_grid {
                fox_grid.near((rabbit.x, rabbit.y), vision, nearby);
                threats.clear();
                threats.extend(nearby.iter().map(|&j| fox_positions[j]));
                threats.extend(self.player);
            }
            if let Some(baby) =
                rabbit.act(&mut self.world, &mut self.rng, &rabbit_params, threats)
            {
                newborns.push(baby);
            }
            if in_cabin(rabbit.x, rabbit.y) {
                self.today.cabin_hours_rabbits += 1;
                if !was_in {
                    self.today.cabin_visits_rabbits += 1;
                }
            }
            if let Some(c) = &cabin {
                if self.world.scraps_at(c.x, c.y) < scraps {
                    self.cabin_log.rabbit_ate_scraps(self.tick);
                }
            }
        }
        self.scratch = scratch;
        self.today.births += newborns.len();
        if !newborns.is_empty() && !self.first_birth_logged {
            self.first_birth_logged = true;
            self.events
                .push(format!("Dia {}: nasceu o primeiro coelho! 🐇", self.day()));
        }
        self.rabbits.extend(newborns);

        // Remover os que morreram, guardando a causa.
        let max_age = self.config.rabbit.max_age_ticks;
        let mut eaten = 0;
        let mut hunger = 0;
        let mut old_age = 0;
        self.rabbits.retain(|r| {
            if r.eaten {
                eaten += 1;
                false
            } else if r.energy <= 0.0 {
                hunger += 1;
                false
            } else if r.age > max_age {
                old_age += 1;
                false
            } else {
                true
            }
        });
        self.today.deaths_predation += eaten;
        self.today.deaths_hunger += hunger;
        self.today.deaths_age += old_age;

        self.today.fox_births += fox_newborns.len();
        if !fox_newborns.is_empty() && !self.first_fox_birth_logged {
            self.first_fox_birth_logged = true;
            self.events
                .push(format!("Dia {}: nasceu a primeira cria de raposa!", self.day()));
        }
        self.foxes.extend(fox_newborns);

        let fox_max_age = self.config.fox.max_age_ticks;
        let mut fox_hunger = 0;
        let mut fox_old_age = 0;
        self.foxes.retain(|f| {
            if f.energy <= 0.0 {
                fox_hunger += 1;
                false
            } else if f.age > fox_max_age {
                fox_old_age += 1;
                false
            } else {
                true
            }
        });
        self.today.fox_deaths_hunger += fox_hunger;
        self.today.fox_deaths_age += fox_old_age;

        self.check_milestones();

        if self.tick % TICKS_PER_DAY == 0 {
            self.close_day();
            if !self.foxes_arrived && self.day() >= self.config.foxes_arrive_day {
                self.foxes_arrive();
            }
            self.leave_cabin_food();
        }
    }

    pub fn run_days(&mut self, days: u64) {
        for _ in 0..days * TICKS_PER_DAY {
            self.step();
        }
    }

    fn check_milestones(&mut self) {
        let n = self.rabbits.len();
        while self.next_milestone < POPULATION_MILESTONES.len()
            && n >= POPULATION_MILESTONES[self.next_milestone]
        {
            self.events.push(format!(
                "Dia {}: a população chegou a {} coelhos! 🎉",
                self.day(),
                POPULATION_MILESTONES[self.next_milestone]
            ));
            self.next_milestone += 1;
        }
        if n == 0 && !self.extinction_logged {
            self.extinction_logged = true;
            self.events
                .push(format!("Dia {}: os coelhos extinguiram-se. 💀", self.day()));
        }
        // As raposas podem desaparecer e voltar por migração, por isso registamos
        // cada vez que desaparecem.
        if self.foxes_present && self.foxes.is_empty() {
            self.events
                .push(format!("Dia {}: já não há raposas no mapa.", self.day()));
        }
        self.foxes_present = !self.foxes.is_empty();
    }

    /// Migração de segurança, verificada uma vez por dia.
    fn check_migration(&mut self) {
        let m = self.config.migration.clone();

        if self.rabbits.len() < m.min_rabbits {
            self.rabbit_low_days += 1;
        } else {
            self.rabbit_low_days = 0;
        }
        if self.rabbit_low_days >= m.days_below && m.min_rabbits > 0 {
            self.rabbit_low_days = 0;
            let (x, y) = self.edge_cell();
            let mut rabbit = Rabbit::newborn(x, y, self.config.rabbit.energy_start);
            rabbit.age = self.config.rabbit.maturity_ticks;
            self.rabbits.push(rabbit);
            self.migrations_rabbits += 1;
            self.today.migrations += 1;
            self.events.push(format!(
                "Dia {}: um coelho chegou pela margem do mapa (migração).",
                self.day()
            ));
        }

        // Antes de as raposas chegarem, a falta delas não é uma extinção.
        if self.foxes_arrived && self.foxes.len() < m.min_foxes {
            self.fox_low_days += 1;
        } else {
            self.fox_low_days = 0;
        }
        if self.fox_low_days >= m.days_below && m.min_foxes > 0 {
            self.fox_low_days = 0;
            let (x, y) = self.edge_cell();
            let mut fox = Fox::newborn(x, y, self.config.fox.energy_start);
            fox.age = self.config.fox.maturity_ticks;
            self.foxes.push(fox);
            self.foxes_present = true;
            self.migrations_foxes += 1;
            self.today.migrations += 1;
            self.events.push(format!(
                "Dia {}: uma raposa chegou pela margem do mapa (migração).",
                self.day()
            ));
        }
    }

    /// Uma célula ao acaso na margem do mapa, longe do jogador.
    fn edge_cell(&mut self) -> (usize, usize) {
        self.spawn_cell(|rng, w, h| match rng.range_usize(4) {
            0 => (rng.range_usize(w), 0),
            1 => (rng.range_usize(w), h - 1),
            2 => (0, rng.range_usize(h)),
            _ => (w - 1, rng.range_usize(h)),
        })
    }

    /// Escolhe com `pick` uma célula para um animal aparecer, a mais de
    /// `SPAWN_CLEAR_RADIUS` do jogador: um animal não pode surgir do nada à
    /// frente dele (GDD 7.9). Sem jogador, a primeira escolha serve, e a
    /// simulação gasta os mesmos números do `Rng` que antes.
    fn spawn_cell(&mut self, pick: impl Fn(&mut Rng, usize, usize) -> (usize, usize)) -> (usize, usize) {
        let (w, h) = (self.world.width, self.world.height);
        let mut cell = pick(&mut self.rng, w, h);
        // Num mapa muito pequeno pode não haver célula longe que chegue:
        // ao fim de algumas tentativas, fica a última.
        for _ in 0..SPAWN_TRIES {
            if far_from(self.player, cell) {
                break;
            }
            cell = pick(&mut self.rng, w, h);
        }
        cell
    }

    fn close_day(&mut self) {
        self.check_migration();
        let t = std::mem::take(&mut self.today);
        self.stats.push(DayStats {
            day: self.day(),
            rabbits: self.rabbits.len(),
            births: t.births,
            deaths_hunger: t.deaths_hunger,
            deaths_age: t.deaths_age,
            deaths_predation: t.deaths_predation,
            foxes: self.foxes.len(),
            fox_births: t.fox_births,
            fox_deaths_hunger: t.fox_deaths_hunger,
            fox_deaths_age: t.fox_deaths_age,
            migrations: t.migrations,
            mean_biomass: self.world.mean_biomass(),
            cabin_visits_rabbits: t.cabin_visits_rabbits,
            cabin_visits_foxes: t.cabin_visits_foxes,
            cabin_hours_rabbits: t.cabin_hours_rabbits,
            cabin_hours_foxes: t.cabin_hours_foxes,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn correr(seed: u64, dias: u64) -> Simulation {
        let mut sim = Simulation::new(Config {
            seed,
            ..Config::default()
        });
        sim.run_days(dias);
        sim
    }

    fn correr_sem_raposas(seed: u64, dias: u64) -> Simulation {
        let mut sim = Simulation::new(Config {
            seed,
            ..Config::without_foxes()
        });
        sim.run_days(dias);
        sim
    }

    #[test]
    fn mesma_seed_da_o_mesmo_resultado() {
        let a = correr(42, 60);
        let b = correr(42, 60);
        let pa: Vec<usize> = a.stats.iter().map(|s| s.rabbits).collect();
        let pb: Vec<usize> = b.stats.iter().map(|s| s.rabbits).collect();
        assert_eq!(pa, pb);
    }

    #[test]
    fn seeds_diferentes_dao_historias_diferentes() {
        let a = correr(1, 60);
        let b = correr(2, 60);
        let pa: Vec<usize> = a.stats.iter().map(|s| s.rabbits).collect();
        let pb: Vec<usize> = b.stats.iter().map(|s| s.rabbits).collect();
        assert_ne!(pa, pb);
    }

    #[test]
    fn um_relatorio_por_dia() {
        let sim = correr(5, 30);
        assert_eq!(sim.stats.len(), 30);
        assert_eq!(sim.day(), 30);
    }

    /// Teste de equilíbrio (GDD 18.6), versão inicial: com várias seeds,
    /// os coelhos não se extinguem nem explodem sem controlo.
    #[test]
    fn equilibrio_com_varias_seeds() {
        let celulas = 24 * 24;
        let limite_maximo = celulas; // mais de 1 coelho por célula seria absurdo
        for seed in 1..=10 {
            let sim = correr_sem_raposas(seed, 150);
            let fim = sim.rabbits.len();
            assert!(fim >= 50, "seed {seed}: população demasiado baixa ({fim})");
            assert!(
                fim < limite_maximo,
                "seed {seed}: população explodiu ({fim})"
            );
        }
    }

    #[test]
    fn a_vegetacao_nao_colapsa() {
        for seed in 1..=10 {
            let sim = correr(seed, 150);
            let media = sim.world.mean_biomass();
            assert!(media > 1.0, "seed {seed}: vegetação colapsou ({media})");
        }
    }

    /// Teste de equilíbrio predador-presa (GDD 18.6): 100 dias com 20 seeds,
    /// a contar da chegada do jogador. Nenhuma espécie se pode extinguir sem a
    /// migração de segurança, e a migração quase nunca deve ser precisa
    /// (contando também as do aquecimento).
    #[test]
    fn equilibrio_predador_presa() {
        let celulas = 24 * 24;
        let mut migracoes = 0;
        for seed in 1..=20 {
            let mut sim = Simulation::warmed_up(Config {
                seed,
                ..Config::default()
            });
            sim.run_days(100);
            migracoes += sim.migrations_rabbits + sim.migrations_foxes;
            let coelhos = sim.rabbits.len();
            assert!(coelhos > 0, "seed {seed}: os coelhos extinguiram-se");
            assert!(coelhos < celulas, "seed {seed}: os coelhos explodiram ({coelhos})");
            assert!(!sim.foxes.is_empty(), "seed {seed}: as raposas extinguiram-se");
        }
        assert!(migracoes <= 2, "a migração de segurança foi precisa {migracoes} vezes");
    }

    #[test]
    fn as_raposas_chegam_no_dia_certo() {
        let mut sim = correr(1, 29);
        assert!(sim.foxes.is_empty());
        assert_eq!(sim.migrations_foxes, 0, "a falta de raposas antes de chegarem não é extinção");
        sim.run_days(1);
        assert_eq!(sim.foxes.len(), Config::default().initial_foxes);
        assert!(sim.events.iter().any(|e| e.contains("chegaram as raposas")));
    }

    #[test]
    fn o_aquecimento_corre_os_dias_antes_do_jogador() {
        let sim = Simulation::warmed_up(Config::default());
        assert_eq!(sim.day(), Config::default().warmup_days);
        assert!(!sim.foxes.is_empty());
        // O jogador chega a um mundo já cheio, não aos 20 coelhos do início.
        assert!(sim.rabbits.len() > 100, "só {} coelhos", sim.rabbits.len());
    }

    #[test]
    fn as_raposas_cacam() {
        let sim = correr(3, 60);
        let cacados: usize = sim.stats.iter().map(|s| s.deaths_predation).sum();
        assert!(cacados > 0);
    }

    /// A cadeia vegetação, coelhos, raposas tem de ter consequências: com
    /// raposas há menos coelhos e, por isso, mais vegetação. A média começa no
    /// dia 100 porque as raposas só chegam no dia 30 e demoram a multiplicar-se;
    /// antes disso a diferença ainda se confunde com o ruído.
    #[test]
    fn as_raposas_reduzem_os_coelhos() {
        let media = |sim: &Simulation| {
            let dias = &sim.stats[100..];
            let coelhos = dias.iter().map(|s| s.rabbits).sum::<usize>() as f32;
            let biomassa = dias.iter().map(|s| s.mean_biomass).sum::<f32>();
            (coelhos / dias.len() as f32, biomassa / dias.len() as f32)
        };
        for seed in 1..=5 {
            let (coelhos_com, bio_com) = media(&correr(seed, 250));
            let (coelhos_sem, bio_sem) = media(&correr_sem_raposas(seed, 250));
            assert!(coelhos_com < coelhos_sem, "seed {seed}: {coelhos_com} vs {coelhos_sem}");
            assert!(bio_com > bio_sem, "seed {seed}: {bio_com} vs {bio_sem}");
        }
    }

    #[test]
    fn migracao_traz_raposas_quando_desaparecem() {
        let mut sim = Simulation::new(Config {
            initial_foxes: 0,
            foxes_arrive_day: 0,
            ..Config::default()
        });
        // Ao fim de 3 dias sem raposas, chega uma pela margem do mapa.
        sim.run_days(2);
        assert!(sim.foxes.is_empty());
        sim.run_days(1);
        assert_eq!(sim.migrations_foxes, 1);
        assert_eq!(sim.foxes.len(), 1);
        assert!(sim.events.iter().any(|e| e.contains("migração")));
    }

    /// Horas passadas na zona da cabana por coelhos e raposas, depois de o
    /// jogador chegar.
    fn horas_na_cabana(seed: u64, comida: f32, dias: u64) -> (usize, usize) {
        let config = Config::default();
        let mut sim = Simulation::warmed_up(Config {
            seed,
            cabin: Some(CabinParams::centered(config.width, config.height, comida)),
            ..config
        });
        let desde = sim.stats.len();
        sim.run_days(dias);
        let dias = &sim.stats[desde..];
        (
            dias.iter().map(|s| s.cabin_hours_rabbits).sum(),
            dias.iter().map(|s| s.cabin_hours_foxes).sum(),
        )
    }

    /// GDD 9.2: a raposa é um predador noturno. De dia descansa e só caça
    /// com muita fome (`day_hunt_below`), por isso quase todas as caçadas
    /// são ao anoitecer e de noite. Antes da noite, cada hora tinha 4%.
    #[test]
    fn as_raposas_cacam_de_noite() {
        let (mut dia, mut total) = (0, 0);
        for seed in 1..=5 {
            let mut sim = Simulation::warmed_up(Config {
                seed,
                ..Config::default()
            });
            for _ in 0..60 * TICKS_PER_DAY {
                sim.step();
                // Uma raposa que acabou de caçar fica saciada por inteiro.
                let full = sim.config.fox.satiety_ticks;
                let caught = sim.foxes.iter().filter(|f| f.satiety == full).count();
                total += caught;
                if sim.light() == 1.0 {
                    dia += caught;
                }
            }
        }
        assert!(total > 100, "só {total} caçadas");
        assert!(dia * 20 < total, "{dia} de {total} caçadas foram de dia");
    }

    /// GDD 7.1: de noite, os coelhos com energia entre `night_rest_above` e
    /// `rest_cap` descansam na toca. Há mais do dobro dos coelhos escondidos
    /// à meia-noite do que ao meio-dia (medido: 24% contra 9%).
    #[test]
    fn de_noite_ha_mais_coelhos_na_toca() {
        let (mut noite, mut dia) = (0.0, 0.0);
        let escondidos = |sim: &Simulation| {
            let n = sim.rabbits.iter().filter(|r| sim.world.is_burrow(r.x, r.y)).count();
            n as f32 / sim.rabbits.len().max(1) as f32
        };
        for seed in 1..=5 {
            let mut sim = Simulation::warmed_up(Config {
                seed,
                ..Config::default()
            });
            for _ in 0..30 * TICKS_PER_DAY {
                sim.step();
                match sim.tick() % TICKS_PER_DAY {
                    0 => noite += escondidos(&sim),
                    12 => dia += escondidos(&sim),
                    _ => {}
                }
            }
        }
        assert!(noite > 2.0 * dia, "na toca: {noite:.2} à meia-noite, {dia:.2} ao meio-dia");
    }

    /// GDD 7.1 e 7.7: as árvores são abrigo. Onde se cortam fica uma clareira:
    /// os coelhos sentem-se expostos e vão comer para outro sítio, por isso lá
    /// há menos coelhos e a erva cresce mais. Medido nas células que as
    /// árvores cortadas abrigavam, contra as mesmas seeds sem corte (com 20
    /// seeds e 30 dias: cerca de -70% de coelhos e mais do dobro da erva).
    #[test]
    fn cortar_arvores_afasta_os_coelhos_e_a_erva_cresce() {
        use crate::resources::ResourceKind;
        let (mut coelhos, mut erva) = ([0.0f64; 2], [0.0f64; 2]);
        for seed in 1..=10 {
            for (i, cortar) in [false, true].into_iter().enumerate() {
                let mut config = Config {
                    seed,
                    ..Config::default()
                };
                config.resources.tree_regrowth = 0.0;
                let mut sim = Simulation::warmed_up(config);
                let arvores: Vec<(usize, usize)> = sim
                    .world
                    .resources()
                    .iter()
                    .filter(|r| r.kind == ResourceKind::Tree && r.x < sim.world.width / 2)
                    .map(|r| (r.x, r.y))
                    .collect();
                let zona: Vec<(usize, usize)> =
                    (0..sim.world.height).flat_map(|y| (0..sim.world.width).map(move |x| (x, y))).filter(|&c| sim.world.tree_cover(c.0, c.1) && c.0 < sim.world.width / 2 + 1).collect();
                if cortar {
                    for &(x, y) in &arvores {
                        sim.world.take_resource(x, y, f32::MAX);
                    }
                }
                for _ in 0..60 * TICKS_PER_DAY {
                    sim.step();
                    if sim.tick() % 6 == 0 {
                        coelhos[i] += sim.rabbits.iter().filter(|r| zona.contains(&(r.x, r.y))).count() as f64;
                        erva[i] += zona.iter().map(|&(x, y)| sim.world.biomass_at(x, y) as f64).sum::<f64>();
                    }
                }
            }
        }
        assert!(coelhos[1] < 0.95 * coelhos[0], "coelhos na zona: {:.0} sem corte, {:.0} com", coelhos[0], coelhos[1]);
        assert!(erva[1] > 1.05 * erva[0], "erva na zona: {:.0} sem corte, {:.0} com", erva[0], erva[1]);
    }

    /// A medição do teste da cabana funciona: com cabana há visitas, sem
    /// cabana não há. (As raposas visitam tão pouco que, numa seed só, podem
    /// ser zero; por isso só se exige que haja visitas de coelhos.)
    #[test]
    fn a_cabana_conta_visitas() {
        let (coelhos, _) = horas_na_cabana(1, 10.0, 30);
        assert!(coelhos > 0);
        // Sem cabana não há zona, por isso não há visitas.
        let sem_cabana: usize = correr(1, 30).stats.iter().map(|s| s.cabin_visits_rabbits).sum();
        assert_eq!(sem_cabana, 0);
    }

    /// A cabana sem comida é a execução de controlo do teste da cabana. Só
    /// muda o terreno: fica numa clareira, sem as árvores e as pedras que a
    /// abrigavam (ver `CabinParams::in_clearing`), e o resto do terreno é o
    /// mesmo. Nunca há restos. Antes das árvores, não mudava nada de todo;
    /// agora as execuções com e sem comida têm as duas a mesma clareira.
    #[test]
    fn a_cabana_sem_comida_so_muda_a_clareira() {
        let config = Config::default();
        let cabana = CabinParams::centered(config.width, config.height, 0.0);
        let mut com = Simulation::new(Config {
            cabin: Some(cabana.clone()),
            ..config.clone()
        });
        let sem = Simulation::new(config);
        assert_eq!(com.world.burrows(), sem.world.burrows());
        let fora: Vec<_> = sem.world.resources().iter().filter(|r| !cabana.in_clearing((r.x, r.y))).cloned().collect();
        assert_eq!(com.world.resources(), &fora[..]);
        assert!(fora.len() < sem.world.resources().len(), "a clareira devia ter tirado alguma coisa");
        for _ in 0..60 * TICKS_PER_DAY {
            com.step();
            assert_eq!(com.world.scraps_at(cabana.x, cabana.y), 0.0);
        }
    }

    /// O teste da cabana (GDD 18.6): com comida, os animais passam mais tempo
    /// na zona do que sem comida. Foi escrito antes do cheiro, e sem cheiro
    /// falhava (×1,00 nos coelhos).
    ///
    /// Mede horas na zona e não entradas: uma raposa que segue o cheiro até à
    /// cabana fica lá a comer os restos e a caçar os coelhos que vêm pela
    /// comida, por isso quase não volta a "entrar". As horas são também o que
    /// o jogador vê: mais animais à volta.
    ///
    /// "Aumenta de forma mensurável" quer dizer aqui: nos coelhos, o total
    /// sobe, e sobe mais com mais comida; nas raposas, que só encontram a
    /// cabana em algumas seeds, o total sobe pelo menos ×1,5.
    ///
    /// Os coelhos só têm de subir em todas as seeds onde nenhuma raposa foi
    /// à cabana. Quando uma raposa descobre a cabana, fica lá a caçar de
    /// noite e os coelhos passam a ir lá menos do que sem comida (com 10
    /// restos por dia, acontece na seed 6). É a cadeia de acontecimentos que
    /// o teste procura, e não um erro.
    #[test]
    fn comida_junto_a_cabana_atrai_animais() {
        let seeds = 1..=10;
        let mut raposas = [0; 2];
        let mut coelhos_sem = 0;
        let mut coelhos_pouca = 0;
        let mut coelhos_muita = 0;
        for seed in seeds {
            let sem = horas_na_cabana(seed, 0.0, 60);
            let pouca = horas_na_cabana(seed, 10.0, 60);
            let muita = horas_na_cabana(seed, 50.0, 60);
            if pouca.1 == 0 {
                assert!(pouca.0 > sem.0, "seed {seed}: coelhos {} h sem comida, {} h com", sem.0, pouca.0);
            }
            coelhos_sem += sem.0;
            coelhos_pouca += pouca.0;
            coelhos_muita += muita.0;
            raposas[0] += sem.1;
            raposas[1] += pouca.1;
        }
        assert!(coelhos_pouca > coelhos_sem, "coelhos: {coelhos_sem} h sem comida, {coelhos_pouca} h com");
        assert!(coelhos_muita > coelhos_pouca, "mais comida devia atrair mais coelhos");
        assert!(
            raposas[1] as f32 >= 1.5 * raposas[0] as f32,
            "raposas: {} h sem comida, {} h com",
            raposas[0],
            raposas[1]
        );
    }

    /// Cadeias coelho → raposa → caça e caçadas junto à cabana, depois de o
    /// jogador chegar.
    fn cadeias_na_cabana(seed: u64, comida: f32, dias: u64) -> (usize, usize) {
        let config = Config::default();
        let mut sim = Simulation::warmed_up(Config {
            seed,
            cabin: Some(CabinParams::centered(config.width, config.height, comida)),
            ..config
        });
        let antes = sim.cabin_log.clone();
        sim.run_days(dias);
        (sim.cabin_log.chains - antes.chains, sim.cabin_log.hunts - antes.hunts)
    }

    /// GDD 18.6: pelo menos uma sequência de eventos encadeados acontece sem
    /// ter sido escrita, e os resultados variam entre seeds. A cadeia é
    /// "um coelho come os restos, uma raposa chega à cabana e caça lá um
    /// coelho, tudo em menos de 2 dias". Nenhuma linha de código a pede: é a
    /// soma do cheiro, da fome e da caça.
    ///
    /// Sem comida não há cadeias por definição (nenhum coelho come restos), por
    /// isso o controlo a sério são as caçadas: com comida tem de haver mais
    /// caçadas junto à cabana do que sem comida.
    #[test]
    fn a_cadeia_da_cabana_acontece_sem_ser_escrita() {
        let com: Vec<(usize, usize)> = (1..=20).map(|seed| cadeias_na_cabana(seed, 10.0, 60)).collect();
        let sem: Vec<(usize, usize)> = (1..=20).map(|seed| cadeias_na_cabana(seed, 0.0, 60)).collect();
        let cadeias: Vec<usize> = com.iter().map(|c| c.0).collect();
        assert!(cadeias.iter().sum::<usize>() > 0, "nenhuma cadeia em 20 seeds");
        assert!(cadeias.iter().any(|&n| n != cadeias[0]), "todas as seeds deram o mesmo: {cadeias:?}");
        assert!(sem.iter().all(|c| c.0 == 0));
        let cacadas = |v: &[(usize, usize)]| v.iter().map(|c| c.1).sum::<usize>();
        assert!(
            cacadas(&com) > cacadas(&sem),
            "caçadas junto à cabana: {} sem comida, {} com",
            cacadas(&sem),
            cacadas(&com)
        );
    }

    /// Muita comida junto à cabana não pode ser uma armadilha: os coelhos
    /// ficavam presos ao cheiro depois de os restos acabarem e morriam de fome
    /// com erva cheia ao lado (com 50 por dia, a população caía para 10).
    #[test]
    fn muita_comida_nao_e_uma_armadilha() {
        let config = Config::default();
        for seed in 1..=5 {
            let mut sim = Simulation::warmed_up(Config {
                seed,
                cabin: Some(CabinParams::centered(config.width, config.height, 200.0)),
                ..config.clone()
            });
            sim.run_days(60);
            let n = sim.rabbits.len();
            assert!(n > 150, "seed {seed}: só {n} coelhos com muita comida");
        }
    }

    /// Animais à vista (coelhos fora da toca e raposas) a `raio` células ou
    /// menos de `centro`. Um coelho escondido numa toca ao lado do jogador
    /// não conta: o jogador não o vê.
    fn animais_perto(sim: &Simulation, centro: (usize, usize), raio: usize) -> usize {
        let coelhos = sim.rabbits.iter().filter(|r| {
            distance((r.x, r.y), centro) <= raio && !sim.world.is_burrow(r.x, r.y)
        });
        let raposas = sim.foxes.iter().filter(|f| distance((f.x, f.y), centro) <= raio);
        coelhos.count() + raposas.count()
    }

    /// GDD 4: o jogador é uma presença no ecossistema. Quando chega, os
    /// animais à volta dele fogem ou escondem-se.
    ///
    /// Mede as primeiras 6 horas, que é o que o jogador vê ao aproximar-se. Com
    /// o jogador parado durante dias, o efeito esbate-se: os coelhos que fogem
    /// não comem, ficam desesperados (`desperate_energy`) e acabam por ir comer
    /// mesmo ao lado dele. É o mesmo custo do medo que já existe com as raposas.
    #[test]
    fn o_jogador_afasta_os_animais() {
        let centro = (12, 12);
        let (mut com, mut sem) = (0, 0);
        for seed in 1..=10 {
            let config = Config {
                seed,
                ..Config::default()
            };
            let mut a = Simulation::warmed_up(config.clone());
            let mut b = Simulation::warmed_up(config);
            a.place_player(Some(centro));
            for _ in 0..6 {
                a.step();
                b.step();
            }
            let (na, nb) = (animais_perto(&a, centro, 2), animais_perto(&b, centro, 2));
            assert!(na <= nb, "seed {seed}: {na} animais à vista com jogador, {nb} sem");
            com += na;
            sem += nb;
        }
        assert!(com * 2 <= sem, "{com} animais à vista com jogador, {sem} sem");
    }

    /// O rasto do jogador fica no mundo depois de ele sair (GDD 7.6, cheiro
    /// humano): depois de dois dias a patrulhar uma zona, as raposas evitam-na
    /// nas horas seguintes, comparado com raposas que não sentem o cheiro.
    #[test]
    fn as_raposas_evitam_o_rasto_do_jogador() {
        // O perímetro de um quadrado de lado 8 à volta de (12, 12).
        let mut rota = Vec::new();
        rota.extend((8..16).map(|x| (x, 8)));
        rota.extend((8..16).map(|y| (16, y)));
        rota.extend((9..=16).rev().map(|x| (x, 16)));
        rota.extend((9..=16).rev().map(|y| (8, y)));
        let horas_na_zona = |seed: u64, olfato: bool| {
            let mut config = Config {
                seed,
                ..Config::default()
            };
            if !olfato {
                config.fox.human_smell_avoid = f32::INFINITY;
            }
            let mut sim = Simulation::warmed_up(config);
            for t in 0..2 * TICKS_PER_DAY as usize {
                sim.place_player(Some(rota[t % rota.len()]));
                sim.step();
            }
            sim.place_player(None);
            let mut horas = 0;
            for _ in 0..12 {
                sim.step();
                horas += sim.foxes.iter().filter(|f| distance((f.x, f.y), (12, 12)) <= 5).count();
            }
            horas
        };
        let com: usize = (1..=20).map(|seed| horas_na_zona(seed, true)).sum();
        let sem: usize = (1..=20).map(|seed| horas_na_zona(seed, false)).sum();
        assert!(com * 2 <= sem, "{com} horas de raposa na zona com olfato, {sem} sem");
    }

    /// As grelhas espaciais só podem mudar a velocidade, nunca o resultado:
    /// num mapa grande, com e sem grelhas, tudo tem de ficar exatamente igual.
    #[test]
    fn as_grelhas_nao_mudam_nada() {
        let mut config = Config::default();
        crate::sweep::sized_map(&mut config, 72);
        config.initial_foxes = 18;
        let mut com = Simulation::new(config.clone());
        let mut sem = Simulation::new(config);
        com.grid_min_pairs = 0;
        sem.grid_min_pairs = usize::MAX;
        com.place_player(Some((30, 30)));
        sem.place_player(Some((30, 30)));
        com.run_days(50);
        sem.run_days(50);
        let resumo = |s: &Simulation| {
            let dias: Vec<(usize, usize, usize)> =
                s.stats.iter().map(|d| (d.rabbits, d.foxes, d.deaths_predation)).collect();
            let raposas: Vec<(usize, usize)> = s.foxes.iter().map(|f| (f.x, f.y)).collect();
            let coelhos: Vec<(usize, usize)> = s.rabbits.iter().map(|r| (r.x, r.y)).collect();
            (dias, raposas, coelhos)
        };
        assert!(com.foxes.len() * com.rabbits.len() > 0, "o teste precisa de raposas e coelhos");
        assert_eq!(resumo(&com), resumo(&sem));
    }

    #[test]
    fn o_jogador_anda_sem_sair_do_mapa() {
        let mut sim = Simulation::new(Config::default());
        sim.move_player(1, 0);
        assert_eq!(sim.player, None, "sem jogador no mapa, não há nada para mexer");
        sim.place_player(Some((0, 0)));
        sim.move_player(-1, -1);
        assert_eq!(sim.player, Some((0, 0)));
        sim.move_player(1, 0);
        sim.move_player(0, 1);
        assert_eq!(sim.player, Some((1, 1)));
    }

    #[test]
    fn sem_vegetacao_os_coelhos_morrem_de_fome() {
        // Sem crescimento: senão a vegetação volta a crescer antes de os
        // coelhos morrerem de fome (seed_growth repõe as células vazias).
        // Sem raposas: senão os coelhos podiam diminuir por serem caçados.
        let mut sim = Simulation::new(Config {
            vegetation: VegetationParams {
                regrowth: 0.0,
                seed_growth: 0.0,
                ..VegetationParams::default()
            },
            ..Config::without_foxes()
        });

        // Esvazia todas as células do mundo
        for y in 0..sim.world.height {
            for x in 0..sim.world.width {
                sim.world.take(x, y, 1000.0);
            }
        }

        let antes = sim.rabbits.len();
        sim.run_days(30);

        assert!(
            sim.rabbits.len() < antes,
            "os coelhos deviam ter diminuído, mas passaram de {antes} para {}",
            sim.rabbits.len()
        );
        // E a causa tem de ser mesmo a fome.
        let mortes_fome: usize = sim.stats.iter().map(|s| s.deaths_hunger).sum();
        assert!(mortes_fome > 0, "nenhum coelho morreu de fome");
    }

    /// Os animais que entram no mapa (as raposas que chegam e os que vêm
    /// por migração) não aparecem ao lado do jogador.
    #[test]
    fn quem_chega_nao_aparece_ao_lado_do_jogador() {
        for seed in 1..=20 {
            let mut sim = Simulation::new(Config {
                seed,
                initial_foxes: 6,
                foxes_arrive_day: 1,
                ..Config::default()
            });
            // Na margem, onde a migração chega.
            let jogador = (0, 12);
            sim.place_player(Some(jogador));
            sim.run_days(1);
            for f in &sim.foxes {
                assert!(distance((f.x, f.y), jogador) > SPAWN_CLEAR_RADIUS, "seed {seed}: chegou em ({}, {})", f.x, f.y);
            }
            sim.foxes.clear();
            sim.run_days(3);
            assert_eq!(sim.migrations_foxes, 1, "seed {seed}");
            for f in &sim.foxes {
                assert!(distance((f.x, f.y), jogador) > SPAWN_CLEAR_RADIUS, "seed {seed}: raposa em ({}, {})", f.x, f.y);
            }
        }
    }
}
