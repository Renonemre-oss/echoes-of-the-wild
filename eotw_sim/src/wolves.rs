//! A alcateia de passagem (GDD 18.2: lobos), à escala real (opção A do
//! relatório "Veados javalis e lobos").
//!
//! Um território de lobo tem 128–350 km² (Białowieża 219 km², Nowak et al.
//! 2008; Yellowstone 310 km², Martin et al. 2018): 90 a 240 vezes a paisagem.
//! O lobo não vive nela. Fora dela há uma alcateia que, de noite, às vezes a
//! atravessa: a 9–20 km por dia, entra 0,04–0,24 vezes por dia (cálculo
//! geométrico do relatório) e anda 1–2 horas lá dentro. Se encontra um grupo
//! de veados ou de javalis, tenta caçar; se mata, fica a comer e deixa o
//! resto para as raposas (`World::add_carcass`). Foge do jogador.
//!
//! Simplificações: o lobo atravessa o rio a nado (os lobos nadam bem) e não
//! caça coelhos; ainda não deixa cheiro nem pegadas.

use serde::{Deserialize, Serialize};

use crate::rng::Rng;
use crate::ungulates::{Herd, Species, UngulateParams};
use crate::world::distance;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackParams {
    /// Visitas por dia (de 24 horas). 0,04–0,24 no cálculo; 0,1 no meio.
    pub visits_per_day: f32,
    /// Horas de noite em que pode entrar, para repartir as visitas.
    pub night_hours: f32,
    /// Lobos na alcateia: 4,0–4,7 nos Cárpatos (Nowak et al. 2008).
    pub size_min: u32,
    pub size_max: u32,
    /// Células andadas por hora a viajar: 9–20 km por dia em ≈10 horas são
    /// 90–200 células; 100.
    pub speed: usize,
    /// Até onde encontra um grupo (pelo cheiro e pela vista).
    pub detect: usize,
    /// Distância a que foge do jogador: 106–248 m (resumo), 18 células.
    pub flight: usize,
    /// Carne comida por lobo por dia: 5,58 kg em Białowieża (Jędrzejewski
    /// et al. 2002, confirmado), e a fração comestível de uma presa.
    pub kg_per_wolf_day: f32,
    pub edible: f32,
    /// Fração da carne que fica para as raposas quando a alcateia se vai.
    pub leftover: f32,
    /// Peso (kg) e preferência de cada classe de presa. A preferência vem
    /// das presas dos Cárpatos (32% crias, 54% fêmeas, 14% machos, contra
    /// 20/41/39 na população viva: Nowak et al. 2005, texto completo) e dos
    /// leitões em Itália (1,8 vezes a sua proporção). Entre espécies, na
    /// Polónia o lobo prefere o veado e evita o javali (0,5).
    pub deer_calf: Prey,
    pub deer_female: Prey,
    pub deer_male: Prey,
    pub piglet: Prey,
    pub boar_adult: Prey,
    pub boar_preference: f32,
    /// A partir de quantos lobos o êxito já não sobe (MacNulty et al. 2012:
    /// estabiliza aos 4 contra o wapiti).
    pub full_pack: u32,
}

/// Uma classe de presa.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Prey {
    pub kg: f32,
    /// Peso na escolha (seleção). Já é a seleção medida nas presas mortas,
    /// por isso o êxito (`success`) tem de ser igual em todas as classes da
    /// mesma espécie: senão a composição das presas ficava torcida duas vezes.
    pub selection: f32,
    /// Êxito de uma tentativa com a alcateia toda (*proposta*; com menos de
    /// `full_pack` lobos desce em proporção).
    pub success: f32,
}

impl Default for PackParams {
    fn default() -> Self {
        Self {
            visits_per_day: 0.1,
            night_hours: 9.0,
            size_min: 4,
            size_max: 5,
            speed: 100,
            detect: 30,
            flight: 18,
            kg_per_wolf_day: 5.58,
            edible: 0.6,
            leftover: 0.2,
            deer_calf: Prey { kg: 30.0, selection: 1.6, success: 0.2 },
            deer_female: Prey { kg: 80.0, selection: 1.3, success: 0.2 },
            deer_male: Prey { kg: 120.0, selection: 0.36, success: 0.2 },
            piglet: Prey { kg: 15.0, selection: 1.8, success: 0.15 },
            boar_adult: Prey { kg: 70.0, selection: 0.3, success: 0.15 },
            boar_preference: 0.5,
            full_pack: 4,
        }
    }
}

impl PackParams {
    fn class(&self, species: Species, young: bool, female: bool) -> Prey {
        match (species, young, female) {
            (Species::Deer, true, _) => self.deer_calf,
            (Species::Deer, false, true) => self.deer_female,
            (Species::Deer, false, false) => self.deer_male,
            (Species::Boar, true, _) => self.piglet,
            (Species::Boar, false, _) => self.boar_adult,
        }
    }
}

/// A alcateia, dentro da paisagem.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pack {
    pub x: usize,
    pub y: usize,
    /// Para onde viaja: um passo em x e em y por célula.
    pub dir: (i32, i32),
    pub wolves: u32,
    /// Horas que ainda fica a comer, e a carne que tem a carcaça.
    pub feeding: u32,
    pub meat: f32,
}

/// O que aconteceu numa hora da alcateia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackEvent {
    Kill { species: Species, young: bool },
    /// Deixou uma carcaça com esta carne.
    LeftCarcass { cell: (usize, usize), kg: f32 },
    /// Saiu da paisagem.
    Gone,
}

impl Pack {
    /// Uma alcateia que entra por uma borda, a andar para o lado de lá.
    pub fn arrive(p: &PackParams, (w, h): (usize, usize), rng: &mut Rng) -> Self {
        let wolves = p.size_min + rng.range_usize((p.size_max - p.size_min + 1) as usize) as u32;
        let along = |n: usize, rng: &mut Rng| rng.range_usize(n);
        let side = rng.range_usize(4);
        let drift = rng.range_usize(3) as i32 - 1;
        let (x, y, dir) = match side {
            0 => (along(w, rng), 0, (drift, 1)),
            1 => (along(w, rng), h - 1, (drift, -1)),
            2 => (0, along(h, rng), (1, drift)),
            _ => (w - 1, along(h, rng), (-1, drift)),
        };
        Self { x, y, dir, wolves, feeding: 0, meat: 0.0 }
    }

    pub fn cell(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// Uma hora dentro da paisagem.
    pub fn act(
        &mut self,
        herds: &mut [Herd],
        p: &PackParams,
        ungulates: (&UngulateParams, &UngulateParams),
        rng: &mut Rng,
        player: (usize, usize),
        size: (usize, usize),
    ) -> Vec<PackEvent> {
        let mut events = Vec::new();
        let me = self.cell();
        let scared = distance(me, player) <= p.flight;
        if self.feeding > 0 && !scared {
            self.feeding -= 1;
            let eaten = self.wolves as f32 * p.kg_per_wolf_day / 24.0;
            self.meat = (self.meat - eaten).max(0.0);
            if self.feeding == 0 {
                events.push(PackEvent::LeftCarcass { cell: me, kg: self.meat });
                self.meat = 0.0;
            }
            return events;
        }
        if self.feeding > 0 {
            // O jogador apareceu: larga a carcaça e vai-se.
            events.push(PackEvent::LeftCarcass { cell: me, kg: self.meat });
            self.feeding = 0;
            self.meat = 0.0;
        }
        if scared {
            let dx = (me.0 as i64 - player.0 as i64).signum() as i32;
            let dy = (me.1 as i64 - player.1 as i64).signum() as i32;
            self.dir = if dx == 0 && dy == 0 { self.dir } else { (dx, dy) };
        } else if let Some(i) = herds
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.members.is_empty() && distance(h.cell(), me) <= p.detect)
            .min_by_key(|(_, h)| distance(h.cell(), me))
            .map(|(i, _)| i)
        {
            if let Some(event) = self.hunt(&mut herds[i], p, ungulates, rng) {
                events.push(event);
                return events;
            }
        }
        // Viaja: `speed` células na direção, e sai quando passa a borda.
        let (w, h) = size;
        let nx = self.x as i64 + self.dir.0 as i64 * p.speed as i64;
        let ny = self.y as i64 + self.dir.1 as i64 * p.speed as i64;
        if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
            events.push(PackEvent::Gone);
        } else {
            (self.x, self.y) = (nx as usize, ny as usize);
        }
        events
    }

    /// Uma tentativa de caça a um grupo: escolhe um animal pela preferência e
    /// tenta. Se falha, o grupo foge.
    fn hunt(
        &mut self,
        herd: &mut Herd,
        p: &PackParams,
        (deer, boar): (&UngulateParams, &UngulateParams),
        rng: &mut Rng,
    ) -> Option<PackEvent> {
        let up = if herd.species == Species::Deer { deer } else { boar };
        let species_weight = if herd.species == Species::Boar { p.boar_preference } else { 1.0 };
        let weights: Vec<f32> = herd
            .members
            .iter()
            .map(|m| p.class(herd.species, m.age_days < up.maturity_days, m.female).selection * species_weight)
            .collect();
        let total: f32 = weights.iter().sum();
        let mut pick = rng.next_f32() * total;
        let mut chosen = weights.len() - 1;
        for (i, w) in weights.iter().enumerate() {
            if pick < *w {
                chosen = i;
                break;
            }
            pick -= w;
        }
        let m = herd.members[chosen];
        let young = m.age_days < up.maturity_days;
        let prey = p.class(herd.species, young, m.female);
        let pack = self.wolves.min(p.full_pack) as f32 / p.full_pack as f32;
        (self.x, self.y) = herd.cell();
        herd.fleeing = 2;
        if rng.next_f32() >= prey.success * pack {
            return None;
        }
        herd.members.remove(chosen);
        let meat = prey.kg * p.edible;
        let per_hour = self.wolves as f32 * p.kg_per_wolf_day / 24.0;
        self.meat = meat;
        self.feeding = ((meat * (1.0 - p.leftover)) / per_hour).ceil() as u32;
        Some(PackEvent::Kill { species: herd.species, young })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ungulates::Member;

    fn grupo(species: Species, cell: (usize, usize), membros: &[(u32, bool)]) -> Herd {
        let members = membros.iter().map(|&(age_days, female)| Member { age_days, female }).collect();
        Herd { species, x: cell.0, y: cell.1, members, goal: None, fleeing: 0 }
    }

    #[test]
    fn a_alcateia_atravessa_e_sai() {
        let p = PackParams::default();
        let mut rng = Rng::new(1);
        let mut a = Pack::arrive(&p, (120, 120), &mut rng);
        let (d, b) = (UngulateParams::deer(), UngulateParams::boar());
        let mut horas = 0;
        loop {
            horas += 1;
            let e = a.act(&mut [], &p, (&d, &b), &mut rng, (1000, 1000), (120, 120));
            if e.contains(&PackEvent::Gone) {
                break;
            }
            assert!(horas < 5);
        }
        assert!(horas <= 2, "1–2 horas dentro da paisagem: {horas}");
    }

    /// As presas mortas seguem as preferências medidas nos Cárpatos: mais
    /// crias e fêmeas do que há na população, poucos machos adultos.
    #[test]
    fn o_lobo_prefere_crias_e_femeas() {
        let p = PackParams::default();
        let (d, b) = (UngulateParams::deer(), UngulateParams::boar());
        let mut rng = Rng::new(7);
        let mut mortos = [0; 3];
        for _ in 0..4000 {
            // A população viva dos Cárpatos: 20% crias, 41% fêmeas, 39% machos.
            let membros: Vec<(u32, bool)> = (0..100)
                .map(|i| if i < 20 { (10, true) } else if i < 61 { (200, true) } else { (200, false) })
                .collect();
            let mut g = [grupo(Species::Deer, (60, 60), &membros)];
            let mut a = Pack { x: 60, y: 50, dir: (0, 1), wolves: 5, feeding: 0, meat: 0.0 };
            for e in a.act(&mut g, &p, (&d, &b), &mut rng, (1000, 1000), (120, 120)) {
                if let PackEvent::Kill { young, .. } = e {
                    let falta = 100 - g[0].members.len();
                    assert_eq!(falta, 1);
                    let i = if young { 0 } else if g[0].members.iter().filter(|m| m.female).count() < 61 { 1 } else { 2 };
                    mortos[i] += 1;
                }
            }
        }
        let n: i32 = mortos.iter().sum();
        let pct = mortos.map(|m| 100 * m / n);
        // Medido: 32% crias, 54% fêmeas, 14% machos. Com ±10 pontos.
        assert!((pct[0] - 32).abs() <= 10 && (pct[1] - 54).abs() <= 10 && (pct[2] - 14).abs() <= 10, "{pct:?}");
    }

    #[test]
    fn come_a_presa_e_deixa_o_resto_as_raposas() {
        let p = PackParams::default();
        let (d, b) = (UngulateParams::deer(), UngulateParams::boar());
        let mut rng = Rng::new(1);
        let mut a = Pack { x: 60, y: 50, dir: (0, 1), wolves: 5, feeding: 0, meat: 0.0 };
        let mut g = [grupo(Species::Deer, (60, 60), &[(10, true)])];
        let mut horas = 0;
        let mut resto = None;
        while resto.is_none() && horas < 200 {
            horas += 1;
            for e in a.act(&mut g, &p, (&d, &b), &mut rng, (1000, 1000), (120, 120)) {
                if let PackEvent::LeftCarcass { kg, .. } = e {
                    resto = Some(kg);
                }
            }
            if g[0].members.is_empty() && a.feeding == 0 && resto.is_none() {
                break;
            }
        }
        let kg = resto.expect("matou a cria e deixou a carcaça");
        let carne = p.deer_calf.kg * p.edible;
        assert!(kg > 0.0 && kg <= p.leftover * carne + 1.0, "deixou {kg} kg de {carne}");
    }

    #[test]
    fn foge_do_jogador_e_larga_a_carcaca() {
        let p = PackParams::default();
        let (d, b) = (UngulateParams::deer(), UngulateParams::boar());
        let mut rng = Rng::new(1);
        let mut a = Pack { x: 60, y: 60, dir: (0, 1), wolves: 5, feeding: 10, meat: 40.0 };
        let e = a.act(&mut [], &p, (&d, &b), &mut rng, (60, 65), (120, 120));
        assert!(matches!(e[0], PackEvent::LeftCarcass { kg, .. } if kg == 40.0));
        assert_eq!(a.dir.1, -1, "vai para longe do jogador");
    }
}
