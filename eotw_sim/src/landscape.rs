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
//! A janela só se desloca quando o jogador já entrou `HYSTERESIS` células
//! na região nova. Assim, andar para trás e para a frente em cima de uma
//! fronteira não desloca a janela a cada passo. A margem da janela, onde as
//! regiões entram e saem, fica sempre a `WINDOW_MARGIN` células ou mais
//! dele: nada aparece nem desaparece à frente dele.
//!
//! Uma vez por dia, alguns animais passam para as regiões vizinhas, à taxa
//! medida na completa (`rabbits_cross` e `foxes_cross`). É isso que deixa uma
//! região vazia voltar a ter coelhos e raposas.

use serde::{Deserialize, Serialize};
use crate::fox::Fox;
use crate::rabbit::Rabbit;
use crate::region::{AggregateParams, AggregateRegion};
use crate::resources::{Craft, Inventory, Item, Resource, Tool};
use crate::rng::Rng;
use crate::scent::KINDS;
use crate::shelter::{Build, Shelter, Stage};
use crate::survival::Needs;
use crate::ungulates::{Ground, Herd, Species};
use crate::wolves::{Pack, PackEvent};
use crate::sim::{CabinParams, Config, Simulation, MIN_REGION_SIZE, SPAWN_CLEAR_RADIUS, TICKS_PER_DAY};
use crate::world::{distance, World};

/// O lado de uma região, em células.
pub const REGION: usize = MIN_REGION_SIZE;

/// Quantas regiões há à volta da do jogador, para cada lado, na janela.
const WINDOW_REACH: usize = 1;

/// Quantas células o jogador pode entrar numa região vizinha sem a janela
/// se deslocar. Sem isto, quem andasse em cima de uma fronteira deslocava a
/// janela a cada passo: cada deslocamento custa 0,6 ms e agrega e expande
/// as regiões que saem e entram.
const HYSTERESIS: usize = 4;

/// O jogador fica sempre a esta distância ou mais da margem da janela (a
/// não ser junto às margens da paisagem, onde já não há mais nada).
pub const WINDOW_MARGIN: usize = REGION - HYSTERESIS;

/// Os animais que passam da janela para uma região agregada saem desta faixa
/// junto à fronteira, e os que chegam aparecem nela.
const BORDER_STRIP: usize = 4;

/// Quantos dias a paisagem corre, só com regiões agregadas, antes de o
/// jogador chegar. As raposas só chegam ao equilíbrio depois de a primeira
/// geração morrer de velhice (400 dias): sem isto, o jogo começava com 4
/// raposas por região em vez de 10 ou 11. Agregadas, as 25 regiões do jogo
/// correm estes dias em cerca de 100 ms.
pub const WARMUP_DAYS: u64 = 600;

/// As sequências de números ao acaso da paisagem (ver `Rng::derived`).
const STREAM_START: u64 = 11;
const STREAM_REGION: u64 = 12;
const STREAM_WINDOW: u64 = 13;
const STREAM_CROSSING: u64 = 14;
const STREAM_RIVER: u64 = 15;
const STREAM_WILD: u64 = 16;

/// O que existe numa região cortada da janela: o terreno, o cheiro e os
/// animais, em coordenadas da região (0 a 23).
struct Cut {
    world: World,
    scent: Vec<[f32; KINDS]>,
    rabbits: Vec<Rabbit>,
    foxes: Vec<Fox>,
}

/// A janela completa: que regiões cobre e a simulação delas.
#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
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
    /// A região no meio da janela. É a do jogador, a não ser que ele tenha
    /// entrado há pouco numa vizinha (ver `HYSTERESIS`).
    centre: (usize, usize),
    /// A cabana, em células da paisagem inteira. Só existe na simulação
    /// quando a sua região está na janela.
    cabin: Option<CabinParams>,
    rng: Rng,
    /// Acontecimentos das simulações completas que já foram desfeitas,
    /// para não se perderem quando a janela se desloca.
    pub events: Vec<String>,
    /// Quantos animais passaram de uma região para outra, desde o início.
    pub crossings: usize,
    /// O que o jogador leva consigo.
    inventory: Inventory,
    /// O abrigo que o jogador construiu, em células da paisagem inteira.
    /// Começa sem nenhum.
    shelter: Option<Shelter>,
    /// A fome e a saúde do jogador.
    needs: Needs,
    /// Os grupos de veados e de javalis, em células da paisagem inteira.
    pub herds: Vec<Herd>,
    /// A alcateia, quando está a atravessar a paisagem.
    pub pack: Option<Pack>,
    /// Quantas vezes a alcateia entrou e quantas presas matou cá dentro.
    pub wolf_visits: usize,
    pub wolf_kills: usize,
    /// Números ao acaso dos veados, javalis e lobos.
    wild_rng: Rng,
}

impl Ground for Landscape {
    fn size(&self) -> (usize, usize) {
        Landscape::size(self)
    }

    fn is_water(&self, cell: (usize, usize)) -> bool {
        Landscape::is_water(self, cell)
    }

    /// Na janela tira da célula; longe, tira da erva média da região.
    fn take_grass(&mut self, cell: (usize, usize), amount: f32) -> f32 {
        if let Some((x, y)) = self.in_window(cell) {
            return self.window.sim.world.take(x, y, amount);
        }
        let (id, _) = self.region_cell(cell);
        let Some(r) = self.regions[id].as_mut() else {
            return 0.0;
        };
        let taken = amount.min(r.biomass);
        r.biomass -= taken / r.cells.max(1) as f32;
        taken
    }

    fn nearest_fruit(&self, from: (usize, usize), radius: usize) -> Option<(usize, usize)> {
        self.bushes()
            .filter(|&(c, f)| f >= 1.0 && distance(c, from) <= radius)
            .min_by_key(|&(c, _)| distance(c, from))
            .map(|(c, _)| c)
    }

    fn take_fruit(&mut self, cell: (usize, usize)) -> f32 {
        if let Some((x, y)) = self.in_window(cell) {
            return self.window.sim.world.take_resource(x, y, f32::MAX);
        }
        let (id, (lx, ly)) = self.region_cell(cell);
        let Some(r) = self.regions[id].as_mut() else {
            return 0.0;
        };
        match r.resources.iter_mut().find(|r| (r.x, r.y) == (lx, ly)) {
            Some(b) => std::mem::take(&mut b.amount),
            None => 0.0,
        }
    }
}

/// A versão do formato dos ficheiros guardados. Sobe quando o que se guarda
/// muda, para um ficheiro antigo dar um erro claro em vez de carregar mal.
pub const SAVE_VERSION: u32 = 5;

#[derive(Serialize)]
struct SaveRef<'a> {
    version: u32,
    landscape: &'a Landscape,
}

#[derive(Deserialize)]
struct SaveFile {
    version: u32,
    landscape: Landscape,
}

/// O resultado de uma tentativa de recolha (`Landscape::gather`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gather {
    /// Recolheu `amount` de `item`. Passaram `gather_hours` de jogo. Se a
    /// ferramenta usada se partiu nesta recolha, `broke` diz qual.
    Got { item: Item, amount: u32, broke: Option<Tool> },
    /// Não há árvore nem pedra ali ao lado, nem erva que chegue para folhas.
    Nothing,
    /// A árvore ou a pedra ao lado já não tem nada para dar.
    Empty(Item),
    /// O inventário está cheio.
    Full,
}

impl Landscape {
    /// Uma paisagem de `cols` x `rows` regiões, com o jogador em `player`.
    /// Cada região começa como um mundo de 24x24 aquecido, com a sua própria
    /// seed, e é logo agregada. Depois a paisagem corre `WARMUP_DAYS` dias
    /// agregada, e só então se expande a janela à volta do jogador.
    /// Uma cabana em `config.cabin` fica nessas células da paisagem inteira.
    pub fn new(config: Config, cols: usize, rows: usize, player: (usize, usize)) -> Self {
        let cabin = config.cabin.clone();
        let config = Config {
            width: REGION,
            height: REGION,
            cabin: None,
            ..config
        };
        let params = AggregateParams::default();
        let mut regions: Vec<Option<AggregateRegion>> = (0..cols * rows)
            .map(|id| {
                let seed = Rng::derived(&[config.seed, id as u64, STREAM_START]).next_u64();
                let sim = Simulation::warmed_up(Config {
                    seed,
                    ..config.clone()
                });
                let mut region = AggregateRegion::from_simulation(&sim, params.clone(), seed);
                // A cabana fica numa clareira, como na simulação de uma região.
                if let Some(c) = &cabin {
                    let (ox, oy) = ((id % cols) * REGION, (id / cols) * REGION);
                    region.resources.retain(|r| !c.in_clearing((ox + r.x, oy + r.y)));
                }
                Some(region)
            })
            .collect();
        let player = (player.0.min(cols * REGION - 1), player.1.min(rows * REGION - 1));
        // O rio é terreno: tira as tocas, as árvores e as pedras de onde passa.
        let mut flow = Rng::derived(&[config.seed, STREAM_RIVER]);
        for (x, y) in config.river.trace(&mut flow, cols * REGION, rows * REGION, player.0) {
            let region = regions[(y / REGION) * cols + x / REGION].as_mut().expect("região");
            let cell = (x % REGION, y % REGION);
            region.water.push(cell);
            region.burrows.retain(|&b| b != cell);
            region.resources.retain(|r| (r.x, r.y) != cell);
        }
        for region in regions.iter_mut().flatten() {
            region.cells = REGION * REGION - region.water.len();
        }
        let rng = Rng::derived(&[config.seed, STREAM_CROSSING]);
        let wild_rng = Rng::derived(&[config.seed, STREAM_WILD]);
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
            centre: (player.0 / REGION, player.1 / REGION),
            cabin,
            rng,
            events: Vec::new(),
            crossings: 0,
            inventory: Inventory::default(),
            shelter: None,
            needs: Needs::default(),
            herds: Vec::new(),
            pack: None,
            wolf_visits: 0,
            wolf_kills: 0,
            wild_rng,
        };
        // Sem janela ainda, todas as regiões são agregadas.
        for _ in 0..WARMUP_DAYS {
            for r in landscape.regions.iter_mut().flatten() {
                r.step_day();
            }
            landscape.cross_borders();
        }
        landscape.crossings = 0;
        landscape.open_window();
        for species in [Species::Deer, Species::Boar] {
            while landscape.animals(species) < landscape.target(species) {
                let cell = landscape.random_land_cell(false);
                let p = landscape.ungulate(species).clone();
                let herd = Herd::new(species, cell, &p, &mut landscape.wild_rng);
                landscape.herds.push(herd);
            }
        }
        landscape
    }

    pub fn ungulate(&self, species: Species) -> &crate::ungulates::UngulateParams {
        match species {
            Species::Deer => &self.config.deer,
            Species::Boar => &self.config.boar,
        }
    }

    /// Quantos animais desta espécie há na paisagem.
    pub fn animals(&self, species: Species) -> usize {
        self.herds.iter().filter(|h| h.species == species).map(|h| h.members.len()).sum()
    }

    /// A área da paisagem: 1 célula = 10 m, 10⁴ células por km².
    fn km2(&self) -> f32 {
        (self.cols * REGION * self.rows * REGION) as f32 / 10_000.0
    }

    /// Quantos haveria à densidade real.
    fn target(&self, species: Species) -> usize {
        (self.ungulate(species).density_km2 * self.km2()).round() as usize
    }

    /// Uma célula de terra ao acaso, longe do jogador; `edge` põe-na numa borda.
    fn random_land_cell(&mut self, edge: bool) -> (usize, usize) {
        let (w, h) = self.size();
        let mut cell = (0, 0);
        for _ in 0..100 {
            let rng = &mut self.wild_rng;
            cell = if edge {
                match rng.range_usize(4) {
                    0 => (rng.range_usize(w), 0),
                    1 => (rng.range_usize(w), h - 1),
                    2 => (0, rng.range_usize(h)),
                    _ => (w - 1, rng.range_usize(h)),
                }
            } else {
                (rng.range_usize(w), rng.range_usize(h))
            };
            if !self.is_water(cell) && distance(cell, self.player) > SPAWN_CLEAR_RADIUS {
                break;
            }
        }
        cell
    }

    /// Uma hora dos veados, dos javalis e da alcateia. No fim de cada dia,
    /// envelhecem, nascem crias, uns grupos saem e outros entram.
    fn step_wild(&mut self) {
        let light = self.window.sim.light();
        let date = self.window.sim.date();
        let fruiting = self.config.calendar.fruiting(date.month);
        let noise = self.window.sim.noise.unwrap_or(0);
        let player = self.player;
        let mut herds = std::mem::take(&mut self.herds);
        let mut rng = std::mem::replace(&mut self.wild_rng, Rng::new(0));
        for herd in &mut herds {
            let p = self.ungulate(herd.species).clone();
            herd.act(self, &p, &mut rng, player, noise, light, fruiting);
        }

        // A alcateia: de noite, às vezes, entra por uma borda.
        let wp = self.config.wolves.clone();
        if self.pack.is_none() && light < 0.2 && rng.next_f32() < wp.visits_per_day / wp.night_hours {
            self.pack = Some(Pack::arrive(&wp, self.size(), &mut rng));
            self.wolf_visits += 1;
        }
        if let Some(mut pack) = self.pack.take() {
            let ungulates = (&self.config.deer, &self.config.boar);
            let events = pack.act(&mut herds, &wp, ungulates, &mut rng, player, self.size());
            let mut gone = false;
            for e in events {
                match e {
                    PackEvent::Kill { species, young } => {
                        self.wolf_kills += 1;
                        let who = match (species, young) {
                            (Species::Deer, true) => "uma cria de veado",
                            (Species::Deer, false) => "um veado",
                            (Species::Boar, true) => "um leitão",
                            (Species::Boar, false) => "um javali",
                        };
                        self.events.push(format!("Dia {}: uma alcateia matou {who}.", self.day()));
                    }
                    PackEvent::LeftCarcass { cell, kg } => {
                        if let Some((x, y)) = self.in_window(cell) {
                            self.window.sim.world.add_carcass(x, y, kg);
                        }
                    }
                    PackEvent::Gone => gone = true,
                }
            }
            if !gone {
                self.pack = Some(pack);
            }
        }

        if self.window.sim.tick() % TICKS_PER_DAY == 0 {
            let first = date.day == 1;
            let km2 = self.km2();
            let count = |s: Species| herds.iter().filter(|h| h.species == s).map(|h| h.members.len()).sum::<usize>();
            let crowding = |s: Species, n: usize| n as f32 / (self.ungulate(s).capacity_km2 * km2);
            let crowded = [crowding(Species::Deer, count(Species::Deer)), crowding(Species::Boar, count(Species::Boar))];
            for herd in &mut herds {
                let p = self.ungulate(herd.species).clone();
                let c = crowded[(herd.species == Species::Boar) as usize];
                herd.day(&p, &mut rng, date.month, first, c);
                if rng.next_f32() < p.leave_per_day {
                    herd.members.clear();
                }
            }
        }
        herds.retain(|h| !h.members.is_empty());
        self.herds = herds;
        self.wild_rng = rng;

        // Entra um grupo novo pela borda por dia, enquanto houver menos do que
        // à densidade real.
        if self.window.sim.tick() % TICKS_PER_DAY == 0 {
            for species in [Species::Deer, Species::Boar] {
                if self.animals(species) < self.target(species) {
                    let cell = self.random_land_cell(true);
                    let p = self.ungulate(species).clone();
                    let herd = Herd::new(species, cell, &p, &mut self.wild_rng);
                    self.herds.push(herd);
                }
            }
        }
    }

    fn size(&self) -> (usize, usize) {
        (self.cols * REGION, self.rows * REGION)
    }

    /// A região agregada e a célula dentro dela, para uma célula fora da janela.
    fn region_cell(&self, (x, y): (usize, usize)) -> (usize, (usize, usize)) {
        ((y / REGION) * self.cols + x / REGION, (x % REGION, y % REGION))
    }

    fn is_water(&self, cell: (usize, usize)) -> bool {
        match self.in_window(cell) {
            Some((x, y)) => self.window.sim.world.is_water(x, y),
            None => {
                let (id, local) = self.region_cell(cell);
                self.regions[id].as_ref().is_some_and(|r| r.water.contains(&local))
            }
        }
    }

    /// Os arbustos com frutos, em células da paisagem.
    fn bushes(&self) -> impl Iterator<Item = ((usize, usize), f32)> + '_ {
        let (ox, oy) = self.window_origin();
        let bush = |r: &&Resource| r.kind == crate::resources::ResourceKind::Bush;
        let window = self.window.sim.world.resources().iter().filter(bush).map(move |r| ((ox + r.x, oy + r.y), r.amount));
        let far = self.regions.iter().enumerate().flat_map(move |(id, r)| {
            let (rx, ry) = ((id % self.cols) * REGION, (id / self.cols) * REGION);
            r.iter().flat_map(|r| r.resources.iter()).filter(bush).map(move |r| ((rx + r.x, ry + r.y), r.amount))
        });
        window.chain(far)
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

    /// A cabana, em células da paisagem inteira, se houver.
    pub fn cabin(&self) -> Option<&CabinParams> {
        self.cabin.as_ref()
    }

    /// Uma célula da paisagem em coordenadas da janela, se estiver nela.
    pub fn in_window(&self, (x, y): (usize, usize)) -> Option<(usize, usize)> {
        let (ox, oy) = self.window_origin();
        let (w, h) = (self.window.cols * REGION, self.window.rows * REGION);
        let (lx, ly) = (x.wrapping_sub(ox), y.wrapping_sub(oy));
        (lx < w && ly < h).then_some((lx, ly))
    }

    pub fn day(&self) -> u64 {
        self.window.sim.day()
    }

    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    /// O mundo todo em texto (JSON), para guardar num ficheiro (GDD 18.2,
    /// Persistência): as regiões agregadas, a janela com cada animal, a
    /// erva, o cheiro, as árvores, o abrigo, o inventário, a fome e a saúde,
    /// e a configuração. Carregado com `load`, continua exatamente como
    /// teria continuado: a mesma seed dá o mesmo resultado.
    pub fn save(&self) -> String {
        serde_json::to_string(&SaveRef { version: SAVE_VERSION, landscape: self })
            .expect("o estado não tem nada que o JSON não guarde")
    }

    pub fn load(text: &str) -> Result<Self, String> {
        let file: SaveFile = serde_json::from_str(text).map_err(|e| format!("o ficheiro não se lê: {e}"))?;
        if file.version != SAVE_VERSION {
            return Err(format!(
                "o ficheiro é da versão {} e este jogo lê a versão {SAVE_VERSION}",
                file.version
            ));
        }
        Ok(file.landscape)
    }

    pub fn needs(&self) -> &Needs {
        &self.needs
    }

    pub fn shelter(&self) -> Option<&Shelter> {
        self.shelter.as_ref()
    }

    /// Recolhe da árvore ou da pedra onde o jogador está, ou de uma das 4
    /// células ao lado (GDD 12, Fase 1). Cada recolha dá `gather_amount`, ou
    /// `tool_amount` com a ferramenta certa (que se gasta), e gasta
    /// `gather_hours` de jogo: o mundo continua enquanto ele trabalha. Se não
    /// houver nada para recolher, ou não couber, não gasta tempo.
    pub fn gather(&mut self) -> Gather {
        let p = self.config.resources.clone();
        let world = &self.window.sim.world;
        let Some(here) = self.in_window(self.player) else {
            return Gather::Nothing;
        };
        let near: Vec<(usize, usize)> = std::iter::once(here)
            .chain(world.neighbors(here.0, here.1).into_iter().flatten())
            .collect();
        let found: Vec<&Resource> = near.iter().filter_map(|&(x, y)| world.resource_at(x, y)).collect();
        let Some(first) = found.first() else {
            return self.gather_leaves(here);
        };
        let Some(r) = found.iter().find(|r| r.amount >= p.gather_amount) else {
            return Gather::Empty(first.kind.item());
        };
        let (item, cell) = (r.kind.item(), (r.x, r.y));
        let tool = Tool::for_item(item).filter(|&t| self.inventory.tool(t).is_some());
        let with_tool = tool.is_some();
        let want = match item {
            Item::Fruit => p.fruit_per_gather,
            _ if with_tool => p.tool_amount,
            _ => p.gather_amount,
        };
        // O que o recurso ainda tem e o que cabe no inventário.
        let space = p.carry.saturating_sub(self.inventory.total());
        let amount = (want.min(r.amount).floor() as u32).min(space);
        if amount == 0 {
            return Gather::Full;
        }
        self.inventory.add(item, amount, p.carry);
        self.window.sim.world.take_resource(cell.0, cell.1, amount as f32);
        let broke = tool.filter(|&t| self.inventory.wear(t));
        let noise = match item {
            Item::Wood => self.config.noise.chop,
            Item::Stone => self.config.noise.mine,
            _ => 0,
        };
        self.work(p.gather_hours, noise);
        Gather::Got { item, amount, broke }
    }

    /// Faz uma ferramenta com a madeira e a pedra do inventário (GDD 12,
    /// Fase 1: "Criar ferramentas básicas"), em qualquer sítio. Gasta
    /// `craft_hours` de jogo; se faltar material, não gasta nada.
    pub fn craft(&mut self, tool: Tool) -> Craft {
        let p = self.config.resources.clone();
        let result = self.inventory.craft(tool, p.tool_uses);
        if result == Craft::Made {
            self.work(p.craft_hours, self.config.noise.craft);
        }
        result
    }

    /// Recolhe folhas e erva seca da célula do jogador, onde não há árvore nem
    /// pedra. Gasta `leaf_biomass` de vegetação por folha, por isso tirar
    /// folhas tira comida aos coelhos.
    fn gather_leaves(&mut self, here: (usize, usize)) -> Gather {
        let p = self.config.resources.clone();
        let per_leaf = self.config.shelter.leaf_biomass;
        if self.window.sim.world.biomass_at(here.0, here.1) < per_leaf {
            return Gather::Nothing;
        }
        // As folhas contam-se inteiras: pelo menos uma por recolha, mesmo que
        // `gather_amount` tenha casas decimais. Só se gasta a erva das que
        // couberam no inventário.
        let want = (p.gather_amount as u32).max(1);
        let added = self.inventory.add(Item::Leaves, want, p.carry);
        if added == 0 {
            return Gather::Full;
        }
        self.window.sim.world.take(here.0, here.1, per_leaf * added as f32);
        for _ in 0..p.gather_hours {
            self.step();
        }
        Gather::Got { item: Item::Leaves, amount: added, broke: None }
    }

    /// Procura comida na erva onde o jogador está (ervas, raízes e
    /// cogumelos): gasta `forage_hours` e `forage_biomass` de vegetação por
    /// porção. Se a erva for pouca ou o inventário estiver cheio, não gasta
    /// tempo.
    pub fn forage(&mut self) -> Gather {
        let p = self.config.needs.clone();
        let carry = self.config.resources.carry;
        let Some(here) = self.in_window(self.player) else {
            return Gather::Nothing;
        };
        if self.window.sim.world.biomass_at(here.0, here.1) < p.forage_biomass {
            return Gather::Nothing;
        }
        if self.inventory.add(Item::Food, 1, carry) == 0 {
            return Gather::Full;
        }
        self.window.sim.world.take(here.0, here.1, p.forage_biomass);
        for _ in 0..p.forage_hours {
            self.step();
        }
        Gather::Got { item: Item::Food, amount: 1, broke: None }
    }

    /// Come um fruto ou, se não houver, uma porção de comida do inventário.
    /// Devolve o que comeu. Não gasta tempo.
    pub fn eat(&mut self) -> Option<Item> {
        let p = &self.config.needs;
        for (item, value) in [(Item::Fruit, p.fruit_meal), (Item::Food, p.meal)] {
            if self.inventory.remove(item, 1) {
                self.needs.eat(value);
                return Some(item);
            }
        }
        None
    }

    /// Constrói o abrigo ou a etapa seguinte dele (ver `shelter.rs`). O
    /// primeiro abrigo fica na célula do jogador e tem de ter uma árvore ao
    /// lado, porque é um abrigo de ramos encostado a ela. As etapas seguintes
    /// constroem-se no mesmo sítio. Gasta o material e as horas da etapa; se
    /// faltar alguma coisa, não gasta nada.
    pub fn build(&mut self) -> Build {
        let p = self.config.shelter.clone();
        let stage = match self.shelter {
            None => {
                let Some(here) = self.in_window(self.player) else {
                    return Build::Blocked;
                };
                let world = &self.window.sim.world;
                if world.resource_at(here.0, here.1).is_some()
                    || world.is_burrow(here.0, here.1)
                    || world.is_water(here.0, here.1)
                {
                    return Build::Blocked;
                }
                // Um toco cortado não aguenta os ramos: tem de ser uma árvore
                // que ainda abriga.
                let tree = world.neighbors(here.0, here.1).into_iter().flatten().any(|(x, y)| world.is_whole_tree(x, y));
                if !tree {
                    return Build::NeedsTree;
                }
                Stage::Ramos
            }
            Some(s) if (s.x, s.y) != self.player => return Build::Elsewhere,
            Some(s) => match s.stage.next() {
                Some(next) => next,
                None => return Build::Finished,
            },
        };
        let cost = p.cost_of(stage);
        if let Some(&(item, n)) = cost.iter().find(|&&(item, n)| self.inventory.count(item) < n) {
            return Build::Missing(stage, item, n - self.inventory.count(item));
        }
        for &(item, n) in cost {
            self.inventory.remove(item, n);
        }
        let (x, y) = self.player;
        self.shelter = Some(Shelter { x, y, stage });
        self.place_player_in_window();
        self.work(p.hours_of(stage), self.config.noise.build);
        Build::Built(stage)
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
    /// A vegetação média de toda a paisagem (terra só), da janela e das
    /// regiões agregadas.
    pub fn mean_biomass(&self) -> f32 {
        let w = &self.window.sim.world;
        let mut sum = w.mean_biomass() * w.land_cells() as f32;
        let mut cells = w.land_cells();
        for r in self.regions.iter().flatten() {
            sum += r.biomass * r.cells as f32;
            cells += r.cells;
        }
        sum / cells.max(1) as f32
    }

    pub fn totals(&self) -> (usize, usize) {
        let mut total = (self.window.sim.rabbits.len(), self.window.sim.foxes.len());
        for r in self.regions.iter().flatten() {
            total.0 += r.rabbit_count() as usize;
            total.1 += r.fox_count() as usize;
        }
        total
    }

    /// Anda uma célula na direção (dx, dy), sem sair da paisagem. Se o
    /// jogador entrar mais de `HYSTERESIS` células numa região vizinha, a
    /// janela desloca-se para ficar com a região dele no meio.
    pub fn move_player(&mut self, dx: i32, dy: i32) {
        let (x, y) = self.player;
        let nx = (x as i32 + dx).clamp(0, (self.cols * REGION) as i32 - 1) as usize;
        let ny = (y as i32 + dy).clamp(0, (self.rows * REGION) as i32 - 1) as usize;
        self.player = (nx, ny);
        let outside = |p: usize, c: usize| {
            let (start, end) = (c * REGION, (c + 1) * REGION);
            start.saturating_sub(p).max((p + 1).saturating_sub(end))
        };
        if outside(nx, self.centre.0).max(outside(ny, self.centre.1)) > HYSTERESIS {
            self.centre = (nx / REGION, ny / REGION);
            self.open_window();
        } else {
            self.place_player_in_window();
        }
    }

    /// Um tick. No fim de cada dia avançam as regiões agregadas e há
    /// animais a mudar de região.
    pub fn step(&mut self) {
        self.window.sim.step();
        self.needs.tick(&self.config.needs);
        self.step_wild();
        if self.window.sim.tick() % TICKS_PER_DAY == 0 {
            for r in self.regions.iter_mut().flatten() {
                r.step_day();
            }
            self.cross_borders();
        }
    }

    /// `hours` de trabalho, a fazer barulho que se ouve até `noise` células
    /// (0 é silencioso; ver `noise.rs`).
    fn work(&mut self, hours: u64, noise: usize) {
        self.window.sim.noise = (noise > 0).then_some(noise);
        for _ in 0..hours {
            self.step();
        }
        self.window.sim.noise = None;
    }

    pub fn run_days(&mut self, days: u64) {
        for _ in 0..days * TICKS_PER_DAY {
            self.step();
        }
    }

    /// Põe o jogador e o abrigo na simulação da janela. O abrigo só cheira
    /// quando está dentro dela.
    fn place_player_in_window(&mut self) {
        let (ox, oy) = self.window_origin();
        self.window.sim.place_player(Some((self.player.0 - ox, self.player.1 - oy)));
        let p = &self.config.shelter;
        self.window.sim.camp = self
            .shelter
            .and_then(|s| self.in_window((s.x, s.y)).map(|cell| (cell, p.scent_of(s.stage))));
    }

    /// As regiões da janela: a do meio e as vizinhas que existem.
    fn window_bounds(&self) -> (usize, usize, usize, usize) {
        let (px, py) = self.centre;
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
        // O registo da cabana e os restos que lá estão continuam na janela nova.
        let cabin_log = std::mem::take(&mut self.window.sim.cabin_log);
        let scraps = self
            .cabin
            .as_ref()
            .and_then(|c| self.in_window((c.x, c.y)))
            .map(|(x, y)| self.window.sim.world.scraps_at(x, y));

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
        let cabin = self.cabin.as_ref().and_then(|c| {
            let (lx, ly) = (c.x.wrapping_sub(rx * REGION), c.y.wrapping_sub(ry * REGION));
            (lx < w && ly < h).then(|| CabinParams { x: lx, y: ly, ..c.clone() })
        });
        let window_config = Config {
            width: w,
            height: h,
            cabin: cabin.clone(),
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
                for (x, y) in cut.world.water_cells() {
                    world.add_water(ox + x, oy + y);
                }
                for &(x, y, kg) in cut.world.carcasses() {
                    world.add_carcass(ox + x, oy + y, kg);
                }
                for r in cut.world.resources() {
                    world.add_resource(Resource { x: ox + r.x, y: oy + r.y, ..r.clone() });
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
        let mut sim = Simulation::from_parts(window_config, rng, world, rabbits, foxes, tick);
        sim.cabin_log = cabin_log;
        if let (Some(c), Some(left)) = (&cabin, scraps) {
            // `from_parts` repõe a comida do dia; ficam os restos que havia.
            sim.world.take_scraps(c.x, c.y, f32::MAX);
            sim.world.add_scraps(c.x, c.y, left);
        }
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
        for r in sim.world.resources().iter().filter(|r| inside(r.x, r.y)) {
            world.add_resource(Resource { x: r.x - x0, y: r.y - y0, ..r.clone() });
        }
        for y in 0..REGION {
            for x in 0..REGION {
                if sim.world.is_water(x0 + x, y0 + y) {
                    world.add_water(x, y);
                }
            }
        }
        for &(x, y, kg) in sim.world.carcasses().iter().filter(|c| inside(c.0, c.1)) {
            world.add_carcass(x - x0, y - y0, kg);
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
        let near = distance((ox + cell.0, oy + cell.1), self.player) <= SPAWN_CLEAR_RADIUS;
        if near || self.window.sim.world.is_water(cell.0, cell.1) {
            // Não aparece à frente do jogador nem no rio: fica na região de
            // onde vinha.
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
    use crate::resources::{ResourceKind, ResourceParams};
    use crate::shelter::ShelterParams;

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
        for _ in 0..=HYSTERESIS {
            l.move_player(1, 0);
        }
        assert_eq!(l.window_origin(), (REGION, 0), "a janela devia ter andado uma região");
        assert_eq!(l.totals(), antes);
        let ficaram: Vec<_> = animais(&l).into_iter().filter(ficam).collect();
        assert_eq!(ficaram, ficavam);
    }

    /// As árvores e as pedras da paisagem inteira, em células dela, por ordem.
    fn recursos(l: &Landscape) -> Vec<(usize, usize, ResourceKind, u32)> {
        let (ox, oy) = l.window_origin();
        let um = |x: usize, y: usize, r: &Resource| (x + r.x, y + r.y, r.kind, (r.amount * 1000.0).round() as u32);
        let mut v: Vec<_> = l.window_sim().world.resources().iter().map(|r| um(ox, oy, r)).collect();
        for ry in 0..l.rows {
            for rx in 0..l.cols {
                if let Some(region) = l.region(rx, ry) {
                    v.extend(region.resources.iter().map(|r| um(rx * REGION, ry * REGION, r)));
                }
            }
        }
        v.sort_by_key(|&(x, y, _, _)| (y, x));
        v
    }

    /// Anda até à célula, um passo de cada vez.
    fn ir_para(l: &mut Landscape, alvo: (usize, usize)) {
        while l.player() != alvo {
            let p = l.player();
            let dx = (alvo.0 as i32 - p.0 as i32).signum();
            let dy = if dx == 0 { (alvo.1 as i32 - p.1 as i32).signum() } else { 0 };
            l.move_player(dx, dy);
        }
    }

    /// Uma célula livre (sem recurso nem toca) ao lado de uma árvore, em
    /// células da paisagem, a mais perto de `de`.
    fn ao_lado_de_arvore(l: &Landscape, de: (usize, usize)) -> (usize, usize) {
        let (ox, oy) = l.window_origin();
        let w = &l.window_sim().world;
        let livre = |(x, y): (usize, usize)| w.resource_at(x, y).is_none() && !w.is_burrow(x, y);
        w.resources()
            .iter()
            .filter(|r| w.is_whole_tree(r.x, r.y))
            .flat_map(|r| w.neighbors(r.x, r.y).into_iter().flatten())
            .filter(|&c| livre(c))
            .map(|(x, y)| (ox + x, oy + y))
            .min_by_key(|&c| distance(c, de))
            .unwrap()
    }

    fn dar(l: &mut Landscape, item: Item, n: u32) {
        l.inventory.add(item, n, u32::MAX);
    }

    /// O primeiro abrigo é de ramos encostados a uma árvore: longe das
    /// árvores não dá, e sem madeira também não.
    #[test]
    fn o_primeiro_abrigo_precisa_de_uma_arvore_e_de_madeira() {
        let mut l = paisagem(3, 3, (36, 36));
        let (ox, oy) = l.window_origin();
        let w = &l.window_sim().world;
        let sem_arvore = (0..w.height)
            .flat_map(|y| (0..w.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let perto = std::iter::once((x, y)).chain(w.neighbors(x, y).into_iter().flatten());
                perto.into_iter().all(|(a, b)| w.resource_at(a, b).is_none()) && !w.is_burrow(x, y)
            })
            .map(|(x, y)| (ox + x, oy + y))
            .unwrap();
        ir_para(&mut l, sem_arvore);
        dar(&mut l, Item::Wood, 6);
        assert_eq!(l.build(), Build::NeedsTree);

        let lado = ao_lado_de_arvore(&l, (36, 36));
        ir_para(&mut l, lado);
        l.inventory.remove(Item::Wood, 2);
        assert_eq!(l.build(), Build::Missing(Stage::Ramos, Item::Wood, 2));
        assert!(l.shelter().is_none());

        dar(&mut l, Item::Wood, 2);
        let tick = l.window_sim().tick();
        assert_eq!(l.build(), Build::Built(Stage::Ramos));
        assert_eq!(l.window_sim().tick(), tick + 2, "o abrigo de ramos leva 2 horas");
        assert_eq!(l.inventory().count(Item::Wood), 0);
        assert_eq!(l.shelter().map(|s| (s.x, s.y, s.stage)), Some((lado.0, lado.1, Stage::Ramos)));
    }

    /// Encostado a um toco não dá: a árvore cortada já não abriga.
    #[test]
    fn o_abrigo_nao_se_encosta_a_um_toco() {
        let mut l = paisagem(3, 3, (36, 36));
        let lado = ao_lado_de_arvore(&l, (36, 36));
        ir_para(&mut l, lado);
        let here = l.in_window(lado).unwrap();
        let world = &mut l.window.sim.world;
        for (x, y) in world.neighbors(here.0, here.1).into_iter().flatten() {
            world.take_resource(x, y, f32::INFINITY);
        }
        dar(&mut l, Item::Wood, 6);
        assert_eq!(l.build(), Build::NeedsTree);
    }

    /// As melhorias fazem-se no mesmo sítio, uma de cada vez, até à lareira.
    #[test]
    fn as_etapas_constroem_se_no_mesmo_sitio() {
        let mut l = paisagem(3, 3, (36, 36));
        let lado = ao_lado_de_arvore(&l, (36, 36));
        ir_para(&mut l, lado);
        dar(&mut l, Item::Wood, 12);
        dar(&mut l, Item::Leaves, 10);
        dar(&mut l, Item::Stone, 14);
        let mut feitas = Vec::new();
        while let Build::Built(stage) = l.build() {
            feitas.push(stage);
        }
        assert_eq!(feitas, Stage::ALL);
        assert_eq!(l.build(), Build::Finished);
        assert_eq!(l.inventory().total(), 0, "gastou tudo o que as quatro etapas pedem");

        l.move_player(1, 0);
        assert_eq!(l.build(), Build::Elsewhere, "só há um abrigo");
    }

    /// O abrigo deixa cheiro humano mesmo sem o jogador lá, e deixa de o pôr
    /// na simulação quando sai da janela.
    #[test]
    fn o_abrigo_cheira_a_gente() {
        let mut l = paisagem(3, 3, (36, 36));
        let lado = ao_lado_de_arvore(&l, (36, 36));
        ir_para(&mut l, lado);
        dar(&mut l, Item::Wood, 6);
        assert_eq!(l.build(), Build::Built(Stage::Ramos));
        let local = l.in_window(lado).unwrap();
        let p = ShelterParams::default();
        assert_eq!(l.window_sim().camp, Some((local, p.scent_of(Stage::Ramos))));
        assert!(l.window_sim().world.scent.at(crate::scent::Scent::Human, local.0, local.1) > 0.0);

        ir_para(&mut l, (lado.0 + 3 * REGION / 2, lado.1));
        if l.in_window(lado).is_none() {
            assert_eq!(l.window_sim().camp, None);
        }
    }

    /// Onde não há árvore nem pedra, recolher dá folhas e gasta erva.
    #[test]
    fn recolher_folhas_gasta_erva() {
        let mut l = paisagem(3, 3, (36, 36));
        let p = ShelterParams::default();
        let (ox, oy) = l.window_origin();
        let w = &l.window_sim().world;
        let erva = (0..w.height)
            .flat_map(|y| (0..w.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let perto = std::iter::once((x, y)).chain(w.neighbors(x, y).into_iter().flatten());
                perto.into_iter().all(|(a, b)| w.resource_at(a, b).is_none()) && w.biomass_at(x, y) >= 2.0 * p.leaf_biomass
            })
            .unwrap();
        ir_para(&mut l, (ox + erva.0, oy + erva.1));
        let antes = l.window_sim().world.biomass_at(erva.0, erva.1);
        assert_eq!(l.gather(), Gather::Got { item: Item::Leaves, amount: 1, broke: None });
        assert_eq!(l.inventory().count(Item::Leaves), 1);
        let depois = l.window_sim().world.biomass_at(erva.0, erva.1);
        assert!(depois < antes - p.leaf_biomass + 0.5, "{antes} -> {depois}");
    }

    /// As folhas contam-se inteiras: com `gather_amount` de 0,5 ainda dá
    /// uma, e quando só cabe parte só se gasta a erva da parte que coube.
    #[test]
    fn as_folhas_que_nao_cabem_nao_gastam_erva() {
        let mut l = paisagem(3, 3, (36, 36));
        let p = ShelterParams::default();
        let (ox, oy) = l.window_origin();
        let w = &l.window_sim().world;
        let erva = (0..w.height)
            .flat_map(|y| (0..w.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let perto = std::iter::once((x, y)).chain(w.neighbors(x, y).into_iter().flatten());
                perto.into_iter().all(|(a, b)| w.resource_at(a, b).is_none()) && w.biomass_at(x, y) >= 4.0 * p.leaf_biomass
            })
            .unwrap();
        ir_para(&mut l, (ox + erva.0, oy + erva.1));
        l.config.resources.gather_amount = 0.5;
        assert_eq!(l.gather(), Gather::Got { item: Item::Leaves, amount: 1, broke: None });

        l.config.resources.gather_amount = 3.0;
        let carry = l.config.resources.carry;
        let resto = carry - l.inventory().total() - 1;
        dar(&mut l, Item::Stone, resto);
        let antes = l.window_sim().world.biomass_at(erva.0, erva.1);
        assert_eq!(l.gather(), Gather::Got { item: Item::Leaves, amount: 1, broke: None });
        let gasto = antes - l.window_sim().world.biomass_at(erva.0, erva.1);
        assert!(gasto < 2.0 * p.leaf_biomass, "gastou {gasto} de erva por uma folha");
        assert_eq!(l.gather(), Gather::Full);
    }

    /// Os arbustos só têm frutos de julho a outubro. Na época, apanhar dá 3
    /// frutos de uma vez, e comer come os frutos primeiro. Sem javalis, que
    /// também os comem (ver `os_javalis_comem_os_frutos`).
    #[test]
    fn os_frutos_apanham_se_na_epoca() {
        let mut config = Config { seed: 2, ..Config::default() };
        config.boar.density_km2 = 0.0;
        let mut l = Landscape::new(config, 3, 3, (36, 36));
        let arbustos = |l: &Landscape| -> Vec<f32> {
            let w = &l.window_sim().world;
            w.resources().iter().filter(|r| r.kind == ResourceKind::Bush).map(|r| r.amount).collect()
        };
        assert!(!arbustos(&l).is_empty());
        while l.window_sim().date().month != 0 {
            l.run_days(1);
        }
        assert!(arbustos(&l).iter().all(|&f| f == 0.0), "em janeiro não há frutos");

        while l.window_sim().date().month != 7 {
            l.run_days(1);
        }
        assert!(arbustos(&l).iter().all(|&f| f > 0.0), "em agosto há frutos");
        let (ox, oy) = l.window_origin();
        let w = &l.window_sim().world;
        let alvo = w
            .resources()
            .iter()
            .filter(|r| r.kind == ResourceKind::Bush)
            .flat_map(|r| w.neighbors(r.x, r.y).into_iter().flatten())
            .find(|&(x, y)| {
                let perto = std::iter::once((x, y)).chain(w.neighbors(x, y).into_iter().flatten());
                perto.filter(|&(a, b)| w.resource_at(a, b).is_some()).count() == 1
            })
            .map(|(x, y)| (ox + x, oy + y))
            .expect("um sítio ao lado de um arbusto, sem mais nada");
        ir_para(&mut l, alvo);
        let p = l.config.resources.clone();
        assert_eq!(l.gather(), Gather::Got { item: Item::Fruit, amount: p.fruit_per_gather as u32, broke: None });
        dar(&mut l, Item::Food, 1);
        let fome = l.needs().food;
        assert_eq!(l.eat(), Some(Item::Fruit));
        assert_eq!(l.needs().food, (fome + l.config.needs.fruit_meal).min(100.0));

        while l.window_sim().date().month != 0 {
            l.run_days(1);
        }
        assert!(arbustos(&l).iter().all(|&f| f == 0.0), "em janeiro já caíram");
    }

    /// GDD 18.2: veados e javalis à densidade real (≈10 e 2,75 por km²):
    /// numa paisagem de 5x5 regiões (1,44 km²), uns 14 veados e 4 javalis.
    /// Entram e saem pelas bordas, mas ficam perto da densidade real, e nunca
    /// estão na água.
    #[test]
    fn veados_e_javalis_a_densidade_real() {
        let mut l = paisagem(1, 5, (60, 60));
        assert_eq!((l.target(Species::Deer), l.target(Species::Boar)), (14, 4));
        let (mut veados, mut javalis, mut dias) = (0, 0, 0);
        for _ in 0..60 {
            l.run_days(1);
            dias += 1;
            veados += l.animals(Species::Deer);
            javalis += l.animals(Species::Boar);
            assert!(l.herds.iter().all(|h| !l.is_water(h.cell())));
        }
        let (v, j) = (veados as f32 / dias as f32, javalis as f32 / dias as f32);
        assert!((10.0..=18.0).contains(&v), "veados: {v}");
        assert!((3.0..=8.0).contains(&j), "javalis: {j}");
    }

    /// Na época, os javalis vão aos arbustos e comem os frutos: competem com
    /// o jogador pela única comida silvestre que ele tem.
    #[test]
    fn os_javalis_comem_os_frutos() {
        let frutos = |boar: f32| {
            let mut config = Config { seed: 4, ..Config::default() };
            config.boar.density_km2 = boar;
            let mut l = Landscape::new(config, 5, 5, (60, 60));
            while l.window_sim().date().month != 8 {
                l.run_days(1);
            }
            l.bushes().map(|(_, f)| f).sum::<f32>()
        };
        let (sem, com) = (frutos(0.0), frutos(10.0));
        assert!(com < 0.9 * sem, "frutos em setembro: {com} com javalis, {sem} sem");
    }

    /// Com a frequência de visita real, a alcateia entra umas 3–4 vezes por
    /// ano de jogo (36 dias) e quase nunca mata cá dentro: é a escala real.
    #[test]
    fn a_alcateia_passa_raramente() {
        let mut l = paisagem(1, 5, (60, 60));
        l.run_days(360);
        let (visitas, mortes) = (l.wolf_visits, l.wolf_kills);
        assert!((15..=60).contains(&visitas), "{visitas} visitas em 10 anos de jogo");
        assert!(mortes <= visitas, "{mortes} presas em {visitas} visitas");
    }

    /// Procurar comida na erva gasta uma hora e erva, e comer tira a fome.
    /// Sem erva não se encontra nada e não passa tempo.
    #[test]
    fn procurar_comida_e_comer() {
        let mut l = paisagem(3, 3, (36, 36));
        let p = l.config.needs.clone();
        let here = l.in_window(l.player()).unwrap();
        l.window.sim.world.set_biomass(here.0, here.1, 0.0);
        let tick = l.window_sim().tick();
        assert_eq!(l.forage(), Gather::Nothing);
        assert_eq!(l.window_sim().tick(), tick);
        assert_eq!(l.eat(), None, "sem comida não se come");

        l.window.sim.world.set_biomass(here.0, here.1, 2.0 * p.forage_biomass);
        let antes = l.window_sim().world.biomass_at(here.0, here.1);
        assert_eq!(l.forage(), Gather::Got { item: Item::Food, amount: 1, broke: None });
        assert_eq!(l.window_sim().tick(), tick + p.forage_hours);
        assert!(l.window_sim().world.biomass_at(here.0, here.1) < antes);

        let fome = l.needs().food;
        assert!(fome < Needs::default().food, "a fome desce com as horas");
        assert_eq!(l.eat(), Some(Item::Food));
        assert_eq!(l.needs().food, (fome + p.meal).min(100.0));
        assert_eq!(l.inventory().count(Item::Food), 0);
    }

    /// Guardar e carregar não muda nada: o mundo carregado continua
    /// exatamente como o original, mesmo depois de a janela se deslocar.
    #[test]
    fn guardar_e_carregar_continua_igual() {
        let mut l = paisagem(3, 3, (36, 36));
        l.run_days(2);
        let lado = ao_lado_de_arvore(&l, (36, 36));
        ir_para(&mut l, lado);
        dar(&mut l, Item::Wood, 6);
        assert_eq!(l.build(), Build::Built(Stage::Ramos));
        l.forage();
        let texto = l.save();
        let Ok(mut c) = Landscape::load(&texto) else { panic!("não carregou") };
        assert_eq!(c.save(), texto);
        for o in [&mut l, &mut c] {
            o.run_days(3);
            for _ in 0..REGION {
                o.move_player(1, 0);
            }
            o.run_days(2);
        }
        assert_eq!(c.save(), l.save());
        assert_eq!(c.shelter(), l.shelter());
        assert_eq!(c.needs(), l.needs());
    }

    #[test]
    fn um_ficheiro_de_outra_versao_da_erro() {
        let l = paisagem(1, 1, (12, 12));
        let texto = l.save().replacen(&format!("\"version\":{SAVE_VERSION}"), "\"version\":999", 1);
        let erro = Landscape::load(&texto).err().unwrap();
        assert!(erro.contains("versão 999"), "{erro}");
        assert!(Landscape::load("isto não é um mundo").is_err());
    }

    /// O rio atravessa a paisagem: não tem erva, nem tocas, árvores ou
    /// pedras, e nenhum animal está nele. Quando a janela anda, fica igual.
    #[test]
    fn o_rio_corta_a_paisagem() {
        let mut l = paisagem(1, 3, (12, 12));
        let agua = |l: &Landscape| {
            let (ox, oy) = l.window_origin();
            let mut v: Vec<_> = l.window_sim().world.water_cells().iter().map(|&(x, y)| (ox + x, oy + y)).collect();
            for ry in 0..l.rows {
                for rx in 0..l.cols {
                    if let Some(r) = l.region(rx, ry) {
                        v.extend(r.water.iter().map(|&(x, y)| (rx * REGION + x, ry * REGION + y)));
                    }
                }
            }
            v.sort();
            v
        };
        let antes = agua(&l);
        assert!(antes.len() > 3 * REGION, "o rio atravessa o mapa: {} células", antes.len());
        for _ in 0..3 {
            l.run_days(1);
            let w = &l.window_sim().world;
            for &(x, y) in &w.water_cells() {
                assert_eq!(w.biomass_at(x, y), 0.0);
                assert!(!w.is_burrow(x, y) && w.resource_at(x, y).is_none());
            }
            assert!(l.window_sim().rabbits.iter().all(|r| !w.is_water(r.x, r.y)));
            assert!(l.window_sim().foxes.iter().all(|f| !w.is_water(f.x, f.y)));
        }
        ir_para(&mut l, (2 * REGION + 12, 2 * REGION + 12));
        assert_eq!(agua(&l), antes);
        let w = &l.window_sim().world;
        assert!(l.window_sim().rabbits.iter().all(|r| !w.is_water(r.x, r.y)));
    }

    /// GDD 19, Etapa 7: o que o jogador muda no mundo fica, quando se afasta
    /// (a região passa a agregada) e volta, e quando guarda e carrega. As
    /// árvores cortadas continuam cortadas (e a crescer), com a clareira à
    /// volta; o abrigo fica onde estava e volta a cheirar a gente; a erva
    /// gasta continua mais baixa do que num mundo igual onde ninguém mexeu.
    #[test]
    fn as_alteracoes_do_jogador_ficam() {
        let mut l = paisagem(1, 5, (12, 12));
        let mut controlo = paisagem(1, 5, (12, 12));
        let lado = ao_lado_de_arvore(&l, (12, 12));
        ir_para(&mut l, lado);
        ir_para(&mut controlo, lado);
        dar(&mut l, Item::Wood, 6);
        assert_eq!(l.build(), Build::Built(Stage::Ramos));
        for _ in 0..l.config.shelter.hours_of(Stage::Ramos) {
            controlo.step();
        }
        // Corta todas as árvores da região 0 e come a erva toda dela.
        let arvores: Vec<(usize, usize)> = l
            .window_sim()
            .world
            .resources()
            .iter()
            .filter(|r| r.kind == ResourceKind::Tree && r.x < REGION && r.y < REGION)
            .map(|r| (r.x, r.y))
            .collect();
        let (ox, oy) = l.window_origin();
        assert_eq!((ox, oy), (0, 0));
        let world = &mut l.window.sim.world;
        for &(x, y) in &arvores {
            world.take_resource(x, y, f32::INFINITY);
        }
        for y in 0..REGION {
            for x in 0..REGION {
                world.set_biomass(x, y, 0.5);
            }
        }
        let clareira = arvores.iter().find_map(|&(x, y)| {
            world.neighbors(x, y).into_iter().flatten().find(|&(a, b)| world.clearing(a, b))
        });
        let clareira = clareira.expect("à volta de uma árvore cortada fica uma clareira");
        let tick = l.window_sim().tick();

        // Vai para longe: a região 0 sai da janela e passa a agregada.
        let longe = (4 * REGION + 12, 12);
        ir_para(&mut l, longe);
        ir_para(&mut controlo, longe);
        assert!(l.region(0, 0).is_some(), "a região 0 devia estar agregada");
        l.run_days(2);
        controlo.run_days(2);
        let erva = |l: &Landscape| l.region(0, 0).unwrap().biomass;
        // Medido: 2,2 contra 4,8.
        assert!(erva(&l) < 0.8 * erva(&controlo), "erva: {} contra {}", erva(&l), erva(&controlo));

        // Guarda e carrega lá longe, e volta.
        let Ok(mut l) = Landscape::load(&l.save()) else { panic!("não carregou") };
        ir_para(&mut l, lado);
        ir_para(&mut controlo, lado);
        assert!(l.region(0, 0).is_none(), "a região 0 devia estar na janela");
        let dias = (l.window_sim().tick() - tick) as f32 / TICKS_PER_DAY as f32;
        let p = &l.config.resources;
        let crescido = (p.tree_regrowth * dias).min(p.tree_wood);
        for &(x, y) in &arvores {
            let r = l.window_sim().world.resource_at(x, y).expect("a árvore cortada continua lá");
            assert!((r.amount - crescido).abs() < 0.05, "a árvore tem {} e devia ter {crescido}", r.amount);
        }
        assert!(l.window_sim().world.clearing(clareira.0, clareira.1), "a clareira continua");
        assert_eq!(l.shelter().map(|s| (s.x, s.y, s.stage)), Some((lado.0, lado.1, Stage::Ramos)));
        l.step();
        let local = l.in_window(lado).unwrap();
        assert!(l.window_sim().world.scent.at(crate::scent::Scent::Human, local.0, local.1) > 0.0);
        let media = |l: &Landscape| {
            let w = &l.window_sim().world;
            (0..REGION * REGION).map(|i| w.biomass_at(i % REGION, i / REGION)).sum::<f32>() / (REGION * REGION) as f32
        };
        assert!(media(&l) < 0.8 * media(&controlo), "erva na volta: {} contra {}", media(&l), media(&controlo));
    }

    /// As árvores, as pedras e os arbustos são terreno: não se perdem nem mudam quando a
    /// janela se desloca, e há as mesmas em cada região. Sem rio, que tira
    /// as que estão no caminho (ver `o_rio_corta_a_paisagem`).
    #[test]
    fn as_arvores_e_as_pedras_ficam_quando_a_janela_anda() {
        let config = Config {
            seed: 1,
            river: crate::river::RiverParams { enabled: false, ..Default::default() },
            ..Config::default()
        };
        let mut l = Landscape::new(config, 4, 4, (47, 36));
        let antes = recursos(&l);
        let p = ResourceParams::default();
        assert_eq!(antes.len(), 16 * (p.trees + p.rocks + p.bushes));
        for _ in 0..=HYSTERESIS {
            l.move_player(1, 0);
        }
        assert_eq!(l.window_origin(), (REGION, 0));
        assert_eq!(recursos(&l), antes);
    }

    /// Ao lado de uma árvore, cada recolha dá 1 madeira e gasta 1 hora.
    #[test]
    fn recolher_madeira_gasta_uma_hora() {
        let mut l = paisagem(3, 3, (36, 36));
        let (ox, oy) = l.window_origin();
        let arvore = l
            .window_sim()
            .world
            .resources()
            .iter()
            .filter(|r| r.kind == ResourceKind::Tree)
            .map(|r| (ox + r.x, oy + r.y))
            .min_by_key(|&c| distance(c, (36, 36)))
            .unwrap();
        // Ao lado da árvore, e não em cima dela.
        let lado = if arvore.0 > 0 { (arvore.0 - 1, arvore.1) } else { (arvore.0 + 1, arvore.1) };
        ir_para(&mut l, lado);
        let tick = l.window_sim().tick();
        let madeira = |l: &Landscape| {
            let local = l.in_window(arvore).unwrap();
            l.window_sim().world.resource_at(local.0, local.1).unwrap().amount
        };
        let antes = madeira(&l);
        assert_eq!(l.gather(), Gather::Got { item: Item::Wood, amount: 1, broke: None });
        assert_eq!(l.inventory().count(Item::Wood), 1);
        assert_eq!(l.window_sim().tick(), tick + 1);
        let p = ResourceParams::default();
        assert!((madeira(&l) - (antes - 1.0 + p.tree_regrowth / 24.0)).abs() < 1e-4);
    }

    /// Com o inventário cheio, ou sem nada ao lado, não recolhe nem gasta tempo.
    #[test]
    fn sem_espaco_ou_sem_recurso_nao_gasta_tempo() {
        let config = Config {
            seed: 3,
            resources: ResourceParams { carry: 1, ..ResourceParams::default() },
            // Sem isto, a erva da célula vazia dava folhas.
            shelter: ShelterParams { leaf_biomass: f32::INFINITY, ..ShelterParams::default() },
            ..Config::default()
        };
        let mut l = Landscape::new(config, 3, 3, (36, 36));
        let (ox, oy) = l.window_origin();
        let world = &l.window_sim().world;
        let r = &world.resources()[0];
        let (alvo, item) = ((ox + r.x, oy + r.y), r.kind.item());
        // Uma célula sem nada nela nem nas 4 ao lado.
        let vazia = (0..world.height)
            .flat_map(|y| (0..world.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                std::iter::once((x, y))
                    .chain(world.neighbors(x, y).into_iter().flatten())
                    .all(|(a, b)| world.resource_at(a, b).is_none())
            })
            .map(|(x, y)| (ox + x, oy + y))
            .unwrap();
        ir_para(&mut l, vazia);
        let tick = l.window_sim().tick();
        assert_eq!(l.gather(), Gather::Nothing);
        ir_para(&mut l, alvo);
        assert_eq!(l.gather(), Gather::Got { item, amount: 1, broke: None });
        assert_eq!(l.gather(), Gather::Full);
        assert_eq!(l.window_sim().tick(), tick + 1, "só a recolha que deu gastou tempo");
    }

    /// Os recursos deste tipo na janela, em células da paisagem, do mais
    /// perto do jogador para o mais longe.
    fn perto(l: &Landscape, kind: ResourceKind, min: f32) -> Vec<(usize, usize)> {
        let (ox, oy) = l.window_origin();
        let mut v: Vec<(usize, usize)> = l
            .window_sim()
            .world
            .resources()
            .iter()
            .filter(|r| r.kind == kind && r.amount >= min)
            .map(|r| (ox + r.x, oy + r.y))
            .collect();
        v.sort_by_key(|&c| distance(c, l.player()));
        v
    }

    /// Recolhe até ter `n` de `item`, de recurso em recurso.
    fn juntar(l: &mut Landscape, kind: ResourceKind, n: u32) {
        let item = kind.item();
        while l.inventory().count(item) < n {
            let alvo = perto(l, kind, 1.0)[0];
            ir_para(l, alvo);
            assert!(matches!(l.gather(), Gather::Got { .. }));
        }
    }

    /// GDD 12, Fase 1: com madeira e pedra faz-se um machado, que corta o
    /// dobro. Sem materiais não se faz nada nem passa tempo.
    #[test]
    fn com_machado_recolhe_o_dobro() {
        let mut l = paisagem(3, 3, (36, 36));
        let tick = l.window_sim().tick();
        assert_eq!(l.craft(Tool::Axe), Craft::Missing(Item::Wood, 3));
        assert_eq!(l.window_sim().tick(), tick);
        juntar(&mut l, ResourceKind::Tree, 3);
        juntar(&mut l, ResourceKind::Rock, 2);
        let tick = l.window_sim().tick();
        assert_eq!(l.craft(Tool::Axe), Craft::Made);
        assert_eq!(l.window_sim().tick(), tick + 1, "fazer o machado gasta 1 hora");
        assert_eq!(l.inventory().tool(Tool::Axe), Some(ResourceParams::default().tool_uses));
        let madeira = l.inventory().count(Item::Wood);
        let arvore = perto(&l, ResourceKind::Tree, 4.0)[0];
        ir_para(&mut l, arvore);
        assert_eq!(l.gather(), Gather::Got { item: Item::Wood, amount: 2, broke: None });
        assert_eq!(l.inventory().count(Item::Wood), madeira + 2);
        assert_eq!(l.inventory().tool(Tool::Axe), Some(ResourceParams::default().tool_uses - 1));
    }

    /// Ao fim de `tool_uses` recolhas a ferramenta parte-se, e a recolha diz-o.
    #[test]
    fn o_machado_parte_se() {
        let config = Config {
            seed: 3,
            resources: ResourceParams { tool_uses: 1, ..ResourceParams::default() },
            ..Config::default()
        };
        let mut l = Landscape::new(config, 3, 3, (36, 36));
        juntar(&mut l, ResourceKind::Tree, 3);
        juntar(&mut l, ResourceKind::Rock, 2);
        assert_eq!(l.craft(Tool::Axe), Craft::Made);
        let arvore = perto(&l, ResourceKind::Tree, 2.0)[0];
        ir_para(&mut l, arvore);
        assert_eq!(l.gather(), Gather::Got { item: Item::Wood, amount: 2, broke: Some(Tool::Axe) });
        assert_eq!(l.inventory().tool(Tool::Axe), None);
    }

    /// Uma árvore cortada volta a crescer, também numa região agregada.
    #[test]
    fn as_arvores_crescem_nas_regioes_agregadas() {
        let mut l = paisagem(1, 4, (12, 12));
        let longe = l.region_mut(3, 3).expect("longe do jogador, agregada");
        let i = longe.resources.iter().position(|r| r.kind == ResourceKind::Tree).unwrap();
        longe.resources[i].amount = 0.0;
        l.run_days(2);
        let p = ResourceParams::default();
        assert_eq!(l.region(3, 3).unwrap().resources[i].amount, 2.0 * p.tree_regrowth);
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

    /// Andar para trás e para a frente em cima de uma fronteira não desloca
    /// a janela. Só se desloca quando o jogador entra mais de `HYSTERESIS`
    /// células na região nova, e depois o mesmo vale para voltar.
    #[test]
    fn andar_em_cima_da_fronteira_nao_desloca_a_janela() {
        let mut l = paisagem(4, 4, (2 * REGION - 1, 36));
        let janela = l.window_origin();
        for _ in 0..10 {
            for _ in 0..HYSTERESIS {
                l.move_player(1, 0);
            }
            for _ in 0..HYSTERESIS {
                l.move_player(-1, 0);
            }
            assert_eq!(l.window_origin(), janela);
        }
        // Mais uma célula e a janela desloca-se.
        for _ in 0..=HYSTERESIS {
            l.move_player(1, 0);
        }
        assert_eq!(l.window_origin(), (janela.0 + REGION, janela.1));
        // De volta à fronteira, do outro lado, não se desloca outra vez.
        for _ in 0..=HYSTERESIS {
            l.move_player(-1, 0);
        }
        assert_eq!(l.player().0, 2 * REGION - 1);
        assert_eq!(l.window_origin(), (janela.0 + REGION, janela.1));
    }

    /// O jogador nunca fica a menos de `WINDOW_MARGIN` células da margem da
    /// janela, a não ser junto às margens da paisagem.
    #[test]
    fn o_jogador_fica_longe_da_margem_da_janela() {
        let lado = 5;
        let mut l = paisagem(5, lado, (60, 60));
        let mut rng = Rng::new(9);
        for _ in 0..3000 {
            let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][rng.range_usize(4)];
            l.move_player(dx, dy);
            let (ox, oy) = l.window_origin();
            let (w, h) = (l.window_sim().world.width, l.window_sim().world.height);
            let (px, py) = l.player();
            let fim = lado * REGION;
            assert!(ox == 0 || px - ox >= WINDOW_MARGIN, "perto da margem esquerda em {:?}", l.player());
            assert!(oy == 0 || py - oy >= WINDOW_MARGIN, "perto da margem de cima em {:?}", l.player());
            assert!(ox + w == fim || ox + w - 1 - px >= WINDOW_MARGIN, "perto da margem direita em {:?}", l.player());
            assert!(oy + h == fim || oy + h - 1 - py >= WINDOW_MARGIN, "perto da margem de baixo em {:?}", l.player());
        }
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

    /// Quando o jogador chega, as raposas já estão em equilíbrio (10 a 11
    /// por região), na janela e nas agregadas. Sem `WARMUP_DAYS` havia 4.
    #[test]
    fn a_paisagem_comeca_em_equilibrio() {
        let (mut janela, mut agregadas, mut n) = (0.0, 0.0, (0.0, 0.0));
        for seed in 1..=4 {
            let l = paisagem(seed, 3, (5, 5));
            assert_eq!(l.day(), Config::default().warmup_days + WARMUP_DAYS);
            for ry in 0..3 {
                for rx in 0..3 {
                    let f = l.counts(rx, ry).1 as f32;
                    if l.region(rx, ry).is_some() {
                        agregadas += f;
                        n.1 += 1.0;
                    } else {
                        janela += f;
                        n.0 += 1.0;
                    }
                }
            }
        }
        for (onde, f) in [("janela", janela / n.0), ("agregadas", agregadas / n.1)] {
            assert!((8.0..14.0).contains(&f), "{onde}: {f:.1} raposas por região");
        }
    }

    /// A paisagem tem, em cada região, o mesmo equilíbrio que uma região
    /// simulada sozinha (cerca de 230 coelhos e 10 raposas), seja na janela
    /// ou agregada. Antes de a raposa ter o gasto e o êxito reais eram cerca
    /// de 280 coelhos e 8 raposas, e o intervalo foi deslocado com eles.
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
            assert!((190.0..280.0).contains(&r), "{onde}: {r:.0} coelhos por região");
            assert!((6.0..15.0).contains(&f), "{onde}: {f:.1} raposas por região");
        }
    }

    /// A cabana fica no mesmo sítio da paisagem quando a janela se desloca,
    /// com os restos e o registo que tinha, e só existe na simulação quando
    /// a sua região está na janela.
    #[test]
    fn a_cabana_fica_no_sitio() {
        let cabana = CabinParams::centered(3 * REGION, 3 * REGION, 10.0);
        let mut l = Landscape::new(
            Config {
                seed: 5,
                cabin: Some(cabana.clone()),
                ..Config::default()
            },
            3,
            3,
            (cabana.x, cabana.y + 1),
        );
        let onde = |l: &Landscape| {
            let c = l.window_sim().config().cabin.as_ref().map(|c| (c.x, c.y));
            c.map(|(x, y)| (l.window_origin().0 + x, l.window_origin().1 + y))
        };
        assert_eq!(onde(&l), Some((cabana.x, cabana.y)));
        l.run_days(3);
        let (lx, ly) = l.in_window((cabana.x, cabana.y)).unwrap();
        let restos = l.window_sim().world.scraps_at(lx, ly);
        let linhas = l.window_sim().cabin_log.lines.len();
        // Um passo para a região de cima: a da cabana continua na janela.
        for _ in 0..REGION / 2 + 2 {
            l.move_player(0, -1);
        }
        assert_eq!(onde(&l), Some((cabana.x, cabana.y)));
        let (lx, ly) = l.in_window((cabana.x, cabana.y)).unwrap();
        assert_eq!(l.window_sim().world.scraps_at(lx, ly), restos);
        assert_eq!(l.window_sim().cabin_log.lines.len(), linhas);
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
