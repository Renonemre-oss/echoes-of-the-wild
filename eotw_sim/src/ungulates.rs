//! Os herbívoros grandes: o veado e o javali (GDD 18.2), num só modelo com
//! dois conjuntos de números.
//!
//! Escala real (opção A do relatório "Veados javalis e lobos"): com células
//! de 10 m, a paisagem de 5x5 regiões tem 1,44 km², e à densidade real leva
//! cerca de 14 veados e 4 javalis. A área vital de cada grupo é maior do que
//! a paisagem, por isso os grupos entram e saem pelas bordas (ver
//! `Landscape::step_herds`). Como são poucos, vivem na paisagem inteira e não
//! passam a números quando estão longe do jogador: comem a erva da célula se
//! estiverem na janela, e a erva média da região se não estiverem.
//!
//! Dois relógios: o comportamento (andar, comer, fugir) corre em horas reais;
//! a demografia (idade, crias, sobrevivência) corre no ano de 36 dias do
//! calendário, com os dias reais × 36/365. Simplificações: ainda não têm
//! fome nem morrem de fome (à densidade real a erva chega e sobra), e o
//! javali ainda não come láparos.

use serde::{Deserialize, Serialize};

use crate::rng::Rng;
use crate::world::distance;

/// Os dias de jogo de um ano (`CalendarParams`: 12 meses de 3 dias).
const DAYS_PER_YEAR: f32 = 36.0;

/// O que um grupo precisa do mundo à sua volta, em células da paisagem.
pub trait Ground {
    fn size(&self) -> (usize, usize);
    fn is_water(&self, cell: (usize, usize)) -> bool;
    /// Tira até `amount` de erva da célula e devolve quanto tirou.
    fn take_grass(&mut self, cell: (usize, usize), amount: f32) -> f32;
    /// O arbusto com frutos mais perto, a `radius` ou menos.
    fn nearest_fruit(&self, from: (usize, usize), radius: usize) -> Option<(usize, usize)>;
    /// Come os frutos todos do arbusto nesta célula.
    fn take_fruit(&mut self, cell: (usize, usize)) -> f32;
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    Deer,
    Boar,
}

impl Species {
    pub fn name(self) -> &'static str {
        match self {
            Species::Deer => "veado",
            Species::Boar => "javali",
        }
    }
}

/// Os números de uma espécie. Fontes e cálculos no relatório "Veados javalis
/// e lobos" (`reports/`); *proposta* quer dizer valor de design.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UngulateParams {
    /// Densidade real (animais por km²). Veado: 4–32/km² (Ueno et al. 2010,
    /// Hokkaido; K = 32,6 em Castilla-La Mancha); 10 é um meio-termo.
    /// Javali: média continental de 2,75/km² (Lewis et al. 2017, 129
    /// estimativas em 5 continentes).
    pub density_km2: f32,
    /// Capacidade de carga (animais por km²): a densidade mais alta medida.
    /// Veado: K = 32,6/km² em Castilla-La Mancha (Carpio Camargo et al. 2021,
    /// resumo). Javali: 11/km² em Collserola (secundária). As crias descem
    /// em linha reta até 0 quando a paisagem chega a este número.
    pub capacity_km2: f32,
    /// Tamanho de um grupo que entra na paisagem. Veado: fêmeas em grupos de
    /// 2–5. Javali: vara de 3 (1–11) em Doñana.
    pub group_min: usize,
    pub group_max: usize,
    /// Células andadas por hora ativa: 2–5 km por dia, em cerca de 10 horas
    /// ativas, são 20–50 células por hora (Laguna et al. 2021, resumo).
    pub step: usize,
    /// Erva comida por animal por dia, em unidades da simulação. Um coelho
    /// come 3,5 unidades e ≈70 g de matéria seca (Cooke 2014), por isso uma
    /// unidade ≈ 20 g: uma fêmea de veado come 2 kg MS = 100 unidades. O
    /// javali come pouca erva (40) e prefere frutos e o que fossa.
    pub grass_per_day: f32,
    /// Horas ativas por dia, para repartir a comida.
    pub active_hours: f32,
    /// Distância de fuga ao jogador, em células (veado ≈150 m, javali ≈90 m;
    /// sem dados ibéricos, ver o relatório), e quanto foge.
    pub flight: usize,
    pub flee_to: usize,
    /// Até onde vê frutos (javali: vai primeiro aos arbustos com frutos).
    pub fruit_sight: usize,
    /// Só o javali fossa: probabilidade, por hora ativa fora da época dos
    /// frutos, de revolver a terra da célula (perde a erva toda).
    pub root_chance: f32,
    /// Atividade: o veado é crepuscular, o javali noturno. Fora das suas
    /// horas está ativo com esta probabilidade (javali sem caça: 0,3,
    /// Podgórski et al. 2013, resumo).
    pub off_hours_active: f32,
    /// Demografia, em dias de jogo (dias reais × 36/365). Veado: primeira
    /// cria aos 2 anos (72 dias), máximo 18 anos (648 dias). Javali: cria
    /// aos ≈10 meses (30 dias), máximo 10 anos (360 dias).
    pub maturity_days: u32,
    pub max_age_days: u32,
    /// O mês dos partos (0 = janeiro, no norte): veado em maio–junho, javali
    /// na primavera.
    pub birth_month: usize,
    /// Probabilidade de uma fêmea adulta ter crias nesse mês, e quantas.
    /// Veado: 1 cria com P = 0,55 por ano. Javali: ninhada de ≈4–5.
    pub birth_chance: f32,
    pub litter: usize,
    /// Sobrevivência anual, das crias e dos adultos (sem lobos nem caça).
    /// Veado: crias 0,7, adultos 0,98. Javali: leitões 0,5, adultos 0,8.
    pub young_survival: f32,
    pub adult_survival: f32,
    /// Probabilidade, por dia de jogo, de um grupo sair da paisagem: a área
    /// vital é ≈4 vezes maior do que ela (*proposta*).
    pub leave_per_day: f32,
}

impl UngulateParams {
    pub fn deer() -> Self {
        Self {
            density_km2: 10.0,
            capacity_km2: 32.0,
            group_min: 2,
            group_max: 5,
            step: 25,
            grass_per_day: 100.0,
            active_hours: 10.0,
            flight: 15,
            flee_to: 40,
            fruit_sight: 0,
            root_chance: 0.0,
            off_hours_active: 0.2,
            maturity_days: 72,
            max_age_days: 648,
            birth_month: 4,
            birth_chance: 0.55,
            litter: 1,
            young_survival: 0.7,
            adult_survival: 0.98,
            leave_per_day: 0.2,
        }
    }

    pub fn boar() -> Self {
        Self {
            density_km2: 2.75,
            capacity_km2: 11.0,
            group_min: 3,
            group_max: 6,
            step: 20,
            grass_per_day: 40.0,
            active_hours: 9.0,
            flight: 9,
            flee_to: 26,
            fruit_sight: 30,
            root_chance: 0.2,
            off_hours_active: 0.3,
            maturity_days: 30,
            max_age_days: 360,
            birth_month: 3,
            birth_chance: 0.8,
            litter: 4,
            young_survival: 0.5,
            adult_survival: 0.8,
            leave_per_day: 0.2,
        }
    }

    /// Está nas suas horas? O veado ao amanhecer e ao anoitecer, o javali de
    /// noite. `light` vai de 0 (noite) a 1 (dia).
    pub fn own_hours(&self, species: Species, light: f32) -> bool {
        match species {
            Species::Deer => light > 0.0 && light < 1.0,
            Species::Boar => light < 0.5,
        }
    }

    /// A sobrevivência anual passada a um dia de jogo.
    fn daily(survival: f32) -> f32 {
        survival.powf(1.0 / DAYS_PER_YEAR)
    }
}

/// Um animal do grupo: a idade em dias de jogo e o sexo.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub age_days: u32,
    pub female: bool,
}

/// Um grupo de veados ou uma vara de javalis, numa célula da paisagem.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Herd {
    pub species: Species,
    pub x: usize,
    pub y: usize,
    pub members: Vec<Member>,
    /// Para onde vai.
    pub goal: Option<(usize, usize)>,
    /// Horas que ainda corre depois de se assustar.
    pub fleeing: u32,
}

impl Herd {
    /// Um grupo novo, com o tamanho e as idades sorteadas.
    pub fn new(species: Species, cell: (usize, usize), p: &UngulateParams, rng: &mut Rng) -> Self {
        let n = p.group_min + rng.range_usize(p.group_max - p.group_min + 1);
        let members = (0..n)
            .map(|_| Member {
                age_days: rng.range_usize(p.max_age_days as usize / 2) as u32,
                female: rng.next_f32() < 0.6,
            })
            .collect();
        Self { species, x: cell.0, y: cell.1, members, goal: None, fleeing: 0 }
    }

    pub fn cell(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// Uma hora. `player` é onde está o jogador, `noise` até onde se ouve o
    /// barulho dele (0 se está calado), `light` a luz e `fruiting` se é a
    /// época dos frutos.
    #[allow(clippy::too_many_arguments)]
    pub fn act(
        &mut self,
        g: &mut impl Ground,
        p: &UngulateParams,
        rng: &mut Rng,
        player: (usize, usize),
        noise: usize,
        light: f32,
        fruiting: bool,
    ) {
        let me = self.cell();
        // Fuge do jogador que vê ou ouve dentro da distância de fuga.
        if distance(me, player) <= p.flight.max(noise.min(p.flight * 2)) {
            self.goal = Some(away(me, player, p.flee_to, g.size()));
            self.fleeing = 2;
        }
        let active = self.fleeing > 0 || p.own_hours(self.species, light) || rng.next_f32() < p.off_hours_active;
        if !active {
            return;
        }
        if self.fleeing == 0 {
            if let Some(bush) = (fruiting && p.fruit_sight > 0).then(|| g.nearest_fruit(me, p.fruit_sight)).flatten() {
                self.goal = Some(bush);
            } else if self.goal.is_none_or(|goal| goal == me) {
                self.goal = Some(wander(me, 40, g.size(), rng));
            }
        }
        self.fleeing = self.fleeing.saturating_sub(1);

        // Anda e come pelo caminho: a erva de uma hora, repartida pelas
        // células por onde passa.
        let goal = self.goal.unwrap_or(me);
        let hunger = self.members.len() as f32 * p.grass_per_day / p.active_hours;
        let steps = p.step;
        let per_cell = hunger / steps as f32;
        let mut eaten = 0.0;
        let mut stuck = true;
        for _ in 0..steps {
            let here = self.cell();
            eaten += g.take_grass(here, per_cell);
            if here == goal {
                stuck = false;
                break;
            }
            match step_toward(g, here, goal, rng) {
                Some(next) => {
                    (self.x, self.y) = next;
                    stuck = false;
                }
                None => break,
            }
        }
        let here = self.cell();
        // Se parou cedo, acaba de comer ali e nas vizinhas.
        let around: Vec<_> = std::iter::once(here).chain(neighbors(g, here)).collect();
        for c in around {
            if eaten >= hunger {
                break;
            }
            eaten += g.take_grass(c, hunger - eaten);
        }
        if self.goal == Some(here) && g.nearest_fruit(here, 0) == Some(here) {
            g.take_fruit(here);
            self.goal = None;
        } else if stuck {
            // Preso na margem do rio, por exemplo: escolhe outro destino.
            self.goal = None;
        }
        if !fruiting && p.root_chance > 0.0 && rng.next_f32() < p.root_chance {
            g.take_grass(here, f32::MAX);
        }
    }

    /// Um dia de jogo: envelhecem, morrem alguns, e no mês dos partos nascem
    /// as crias, menos quando a espécie está perto da capacidade de carga
    /// (`crowding` = animais / capacidade). Devolve quantos nasceram.
    pub fn day(
        &mut self,
        p: &UngulateParams,
        rng: &mut Rng,
        month: usize,
        first_day_of_month: bool,
        crowding: f32,
    ) -> usize {
        let (young, adult) = (UngulateParams::daily(p.young_survival), UngulateParams::daily(p.adult_survival));
        self.members.retain_mut(|m| {
            m.age_days += 1;
            let s = if m.age_days < p.maturity_days { young } else { adult };
            m.age_days <= p.max_age_days && rng.next_f32() < s
        });
        let mut born = 0;
        if first_day_of_month && month == p.birth_month {
            let mothers = self.members.iter().filter(|m| m.female && m.age_days >= p.maturity_days).count();
            let chance = p.birth_chance * (1.0 - crowding).clamp(0.0, 1.0);
            for _ in 0..mothers {
                if rng.next_f32() < chance {
                    born += p.litter;
                }
            }
            for _ in 0..born {
                self.members.push(Member { age_days: 0, female: rng.next_f32() < 0.5 });
            }
        }
        born
    }
}

/// Um passo para a vizinha que mais aproxima de `goal`, sem entrar na água.
/// `None` se nenhuma aproxima.
fn step_toward(g: &impl Ground, from: (usize, usize), goal: (usize, usize), rng: &mut Rng) -> Option<(usize, usize)> {
    let now = distance(from, goal);
    let mut best = None;
    let mut best_score = f32::MIN;
    for c in neighbors(g, from) {
        let d = distance(c, goal);
        if d >= now {
            continue;
        }
        let s = -(d as f32) + rng.range_f32(0.0, 0.5);
        if s > best_score {
            best_score = s;
            best = Some(c);
        }
    }
    best
}

/// As 4 vizinhas que existem e não são água.
fn neighbors(g: &impl Ground, (x, y): (usize, usize)) -> impl Iterator<Item = (usize, usize)> + '_ {
    let (w, h) = g.size();
    [
        (y > 0).then(|| (x, y - 1)),
        (y + 1 < h).then(|| (x, y + 1)),
        (x > 0).then(|| (x - 1, y)),
        (x + 1 < w).then(|| (x + 1, y)),
    ]
    .into_iter()
    .flatten()
    .filter(move |&c| !g.is_water(c))
}

/// Um ponto a `dist` células de `from`, do lado contrário de `threat`.
fn away(from: (usize, usize), threat: (usize, usize), dist: usize, (w, h): (usize, usize)) -> (usize, usize) {
    let dx = from.0 as i64 - threat.0 as i64;
    let dy = from.1 as i64 - threat.1 as i64;
    let len = (dx.abs() + dy.abs()).max(1);
    let x = from.0 as i64 + dx * dist as i64 / len;
    let y = from.1 as i64 + dy * dist as i64 / len;
    (x.clamp(0, w as i64 - 1) as usize, y.clamp(0, h as i64 - 1) as usize)
}

/// Um destino ao acaso a `radius` ou menos.
fn wander(from: (usize, usize), radius: usize, (w, h): (usize, usize), rng: &mut Rng) -> (usize, usize) {
    let r = radius as i64;
    let x = from.0 as i64 + rng.range_usize(2 * radius + 1) as i64 - r;
    let y = from.1 as i64 + rng.range_usize(2 * radius + 1) as i64 - r;
    (x.clamp(0, w as i64 - 1) as usize, y.clamp(0, h as i64 - 1) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um mundo de teste: erva por todo o lado, um rio numa coluna com um vau,
    /// e um arbusto com frutos.
    struct Teste {
        w: usize,
        h: usize,
        erva: Vec<f32>,
        rio: Option<(usize, usize)>,
        arbusto: Option<((usize, usize), f32)>,
    }

    impl Teste {
        fn novo(w: usize, h: usize) -> Self {
            Self { w, h, erva: vec![10.0; w * h], rio: None, arbusto: None }
        }
    }

    impl Ground for Teste {
        fn size(&self) -> (usize, usize) {
            (self.w, self.h)
        }
        fn is_water(&self, (x, y): (usize, usize)) -> bool {
            self.rio.is_some_and(|(col, vau)| x == col && y != vau)
        }
        fn take_grass(&mut self, (x, y): (usize, usize), amount: f32) -> f32 {
            let e = &mut self.erva[y * self.w + x];
            let t = amount.min(*e);
            *e -= t;
            t
        }
        fn nearest_fruit(&self, from: (usize, usize), radius: usize) -> Option<(usize, usize)> {
            self.arbusto.filter(|&(c, f)| f > 0.0 && distance(c, from) <= radius).map(|(c, _)| c)
        }
        fn take_fruit(&mut self, cell: (usize, usize)) -> f32 {
            match &mut self.arbusto {
                Some((c, f)) if *c == cell => std::mem::take(f),
                _ => 0.0,
            }
        }
    }

    fn grupo(species: Species, cell: (usize, usize), n: usize) -> Herd {
        let members = (0..n).map(|_| Member { age_days: 100, female: true }).collect();
        Herd { species, x: cell.0, y: cell.1, members, goal: None, fleeing: 0 }
    }

    #[test]
    fn o_veado_foge_do_jogador_dentro_da_distancia_de_fuga() {
        let p = UngulateParams::deer();
        let mut g = Teste::novo(120, 120);
        let mut rng = Rng::new(1);
        let mut v = grupo(Species::Deer, (60, 60), 3);
        v.act(&mut g, &p, &mut rng, (60 + p.flight + 30, 60), 0, 1.0, false);
        assert_eq!(v.fleeing, 0, "longe do jogador não se assusta");
        let x = v.x;
        v.act(&mut g, &p, &mut rng, (x + p.flight, 60), 0, 1.0, false);
        assert!(v.x + 10 < x, "foge para longe do jogador: {:?}", v.cell());
    }

    #[test]
    fn o_barulho_espanta_o_javali_que_ja_nao_ve_o_jogador() {
        let p = UngulateParams::boar();
        let mut g = Teste::novo(120, 120);
        let mut rng = Rng::new(1);
        let mut j = grupo(Species::Boar, (60, 60), 3);
        let jogador = (60, 60 + p.flight + 1);
        j.act(&mut g, &p, &mut rng, jogador, 0, 1.0, false);
        assert_eq!(j.fleeing, 0, "não o vê");
        let y = j.y;
        let jogador = (j.x, y + p.flight + 1);
        j.act(&mut g, &p, &mut rng, jogador, 10, 1.0, false);
        assert!(j.y + 10 < y, "o machado ouve-se a 10 células: {:?}", j.cell());
    }

    #[test]
    fn o_javali_vai_aos_frutos_e_come_os_todos() {
        let p = UngulateParams::boar();
        let mut g = Teste::novo(120, 120);
        g.arbusto = Some(((70, 60), 6.0));
        let mut rng = Rng::new(1);
        let mut j = grupo(Species::Boar, (60, 60), 4);
        j.act(&mut g, &p, &mut rng, (0, 0), 0, 0.0, true);
        assert_eq!(j.cell(), (70, 60));
        assert_eq!(g.arbusto.unwrap().1, 0.0, "esvazia o arbusto");
    }

    #[test]
    fn so_atravessam_o_rio_no_vau() {
        let p = UngulateParams::deer();
        let mut g = Teste::novo(120, 120);
        g.rio = Some((60, 10));
        let mut rng = Rng::new(3);
        let mut v = grupo(Species::Deer, (55, 80), 2);
        for _ in 0..2000 {
            let antes = v.cell();
            v.act(&mut g, &p, &mut rng, (0, 0), 0, 0.5, false);
            assert!(!g.is_water(v.cell()));
            // Passar de um lado ao outro do rio só pode ser pelo vau.
            if (antes.0 < 60) != (v.x < 60) && v.x != 60 && antes.0 != 60 {
                panic!("saltou o rio de {antes:?} para {:?}", v.cell());
            }
        }
    }

    #[test]
    fn comem_a_erva_que_a_pesquisa_diz() {
        let p = UngulateParams::deer();
        let mut g = Teste::novo(240, 240);
        let mut rng = Rng::new(2);
        let mut v = grupo(Species::Deer, (120, 120), 4);
        let antes: f32 = g.erva.iter().sum();
        for _ in 0..p.active_hours as usize {
            v.act(&mut g, &p, &mut rng, (0, 0), 0, 0.5, false);
        }
        let comido = antes - g.erva.iter().sum::<f32>();
        let esperado = 4.0 * p.grass_per_day;
        assert!((comido - esperado).abs() < 0.05 * esperado, "comeram {comido}, deviam {esperado}");
    }

    #[test]
    fn nascem_crias_no_mes_certo_e_os_velhos_morrem() {
        let p = UngulateParams::boar();
        let mut rng = Rng::new(1);
        let mut j = grupo(Species::Boar, (5, 5), 4);
        assert_eq!(j.day(&p, &mut rng, p.birth_month + 1, true, 0.0), 0);
        let n = j.day(&p, &mut rng, p.birth_month, true, 0.0);
        assert!(n > 0 && n % p.litter == 0, "{n}");
        j.members[0].age_days = p.max_age_days;
        let antes = j.members.len();
        j.day(&p, &mut rng, 0, false, 0.0);
        assert!(j.members.len() < antes);
        let mut cheio = grupo(Species::Boar, (5, 5), 4);
        assert_eq!(cheio.day(&p, &mut rng, p.birth_month, true, 1.0), 0, "na capacidade de carga não nascem");
    }

    #[test]
    fn a_sobrevivencia_anual_bate_certo_em_36_dias() {
        let s = UngulateParams::daily(0.8).powi(36);
        assert!((s - 0.8).abs() < 1e-4, "{s}");
    }
}
