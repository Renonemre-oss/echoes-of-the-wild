//! O mundo: uma grelha de células, cada uma com uma quantidade de vegetação (biomassa).
//!
//! GDD 7.7: a vegetação é um recurso que se gasta e se regenera, não só decoração.

use crate::rng::Rng;
use crate::scent::ScentField;

#[derive(Clone, Debug)]
pub struct VegetationParams {
    /// Biomassa máxima de uma célula.
    pub capacity: f32,
    /// Velocidade de crescimento proporcional à biomassa que já existe.
    pub regrowth: f32,
    /// Pequeno crescimento que existe mesmo com biomassa zero (sementes, raízes).
    /// Sem isto, uma célula totalmente comida nunca recuperava.
    pub seed_growth: f32,
}

impl Default for VegetationParams {
    fn default() -> Self {
        Self {
            capacity: 10.0,
            regrowth: 0.02,
            seed_growth: 0.004,
        }
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub width: usize,
    pub height: usize,
    biomass: Vec<f32>,
    /// Células com toca de coelho. Dentro de uma toca o coelho está a salvo (GDD 7.8).
    burrows: Vec<bool>,
    /// As mesmas tocas em lista, por ordem de linha, para procurar a mais próxima
    /// sem percorrer o mapa todo.
    burrow_list: Vec<(usize, usize)>,
    /// Restos de comida deixados pelo jogador (teste da cabana, GDD 18.4).
    /// Coelhos e raposas comem-nos se estiverem na mesma célula.
    scraps: Vec<f32>,
    /// Os campos de cheiro (GDD 7.6). A simulação deposita e espalha; os
    /// animais só os leem.
    pub scent: ScentField,
    params: VegetationParams,
}

impl World {
    pub fn new(width: usize, height: usize, params: VegetationParams, rng: &mut Rng) -> Self {
        let biomass = (0..width * height)
            .map(|_| rng.range_f32(0.4, 1.0) * params.capacity)
            .collect();
        Self {
            width,
            height,
            biomass,
            burrows: vec![false; width * height],
            burrow_list: Vec::new(),
            scraps: vec![0.0; width * height],
            scent: ScentField::new(width, height),
            params,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn capacity(&self) -> f32 {
        self.params.capacity
    }

    pub fn biomass_at(&self, x: usize, y: usize) -> f32 {
        self.biomass[self.idx(x, y)]
    }

    /// Retira até `amount` de biomassa da célula e devolve o que foi realmente retirado.
    pub fn take(&mut self, x: usize, y: usize, amount: f32) -> f32 {
        let i = self.idx(x, y);
        let taken = amount.min(self.biomass[i]);
        self.biomass[i] -= taken;
        taken
    }

    /// Um passo de crescimento (curva logística): cresce devagar quando está muito
    /// baixa, mais depressa a meio, e estabiliza perto da capacidade.
    pub fn grow(&mut self) {
        let k = self.params.capacity;
        let regrowth = self.params.regrowth;
        let seed_growth = self.params.seed_growth;
        for b in &mut self.biomass {
            let room = 1.0 - *b / k;
            *b += (regrowth * *b + seed_growth * k) * room;
            *b = b.clamp(0.0, k);
        }
    }

    pub fn add_burrow(&mut self, x: usize, y: usize) {
        let i = self.idx(x, y);
        if self.burrows[i] {
            return;
        }
        self.burrows[i] = true;
        // Mantém a lista ordenada por linha e coluna, para os empates darem
        // sempre a mesma toca.
        let pos = self.burrow_list.partition_point(|&(bx, by)| (by, bx) < (y, x));
        self.burrow_list.insert(pos, (x, y));
    }

    pub fn is_burrow(&self, x: usize, y: usize) -> bool {
        self.burrows[self.idx(x, y)]
    }

    /// A toca mais próxima a `max` células ou menos, se houver alguma.
    pub fn nearest_burrow(&self, from: (usize, usize), max: usize) -> Option<(usize, usize)> {
        let mut best: Option<((usize, usize), usize)> = None;
        for &cell in &self.burrow_list {
            let d = distance(from, cell);
            if d <= max && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((cell, d));
            }
        }
        best.map(|(cell, _)| cell)
    }

    pub fn add_scraps(&mut self, x: usize, y: usize, amount: f32) {
        let i = self.idx(x, y);
        self.scraps[i] += amount;
    }

    pub fn scraps_at(&self, x: usize, y: usize) -> f32 {
        self.scraps[self.idx(x, y)]
    }

    /// Retira até `amount` de restos da célula e devolve o que foi realmente retirado.
    pub fn take_scraps(&mut self, x: usize, y: usize, amount: f32) -> f32 {
        let i = self.idx(x, y);
        let taken = amount.min(self.scraps[i]);
        self.scraps[i] -= taken;
        taken
    }

    /// Restos de comida em todas as células a `radius` passos ou menos.
    pub fn scraps_within(&self, from: (usize, usize), radius: usize) -> f32 {
        let (x0, x1) = (from.0.saturating_sub(radius), (from.0 + radius).min(self.width - 1));
        let (y0, y1) = (from.1.saturating_sub(radius), (from.1 + radius).min(self.height - 1));
        let mut total = 0.0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if distance(from, (x, y)) <= radius {
                    total += self.scraps[self.idx(x, y)];
                }
            }
        }
        total
    }

    pub fn mean_biomass(&self) -> f32 {
        self.biomass.iter().sum::<f32>() / self.biomass.len() as f32
    }

    /// As (até) 4 células vizinhas: cima, baixo, esquerda, direita.
    /// Devolve `None` nas posições que ficam fora do mapa.
    pub fn neighbors(&self, x: usize, y: usize) -> [Option<(usize, usize)>; 4] {
        [
            if y > 0 { Some((x, y - 1)) } else { None },
            if y + 1 < self.height { Some((x, y + 1)) } else { None },
            if x > 0 { Some((x - 1, y)) } else { None },
            if x + 1 < self.width { Some((x + 1, y)) } else { None },
        ]
    }
}

/// Distância em passos numa grelha sem diagonais (distância de Manhattan).
pub fn distance(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

/// O vizinho que mais aproxima `from` de `target`. O ruído desempata entre
/// caminhos igualmente bons, para o animal não andar sempre em "L".
pub fn step_toward(
    world: &World,
    rng: &mut Rng,
    from: (usize, usize),
    target: (usize, usize),
) -> (usize, usize) {
    best_neighbor(world, rng, from, |cell| -(distance(cell, target) as f32))
}

/// O vizinho que mais afasta `from` de `threat`.
pub fn step_away(
    world: &World,
    rng: &mut Rng,
    from: (usize, usize),
    threat: (usize, usize),
) -> (usize, usize) {
    best_neighbor(world, rng, from, |cell| distance(cell, threat) as f32)
}

fn best_neighbor(
    world: &World,
    rng: &mut Rng,
    from: (usize, usize),
    score: impl Fn((usize, usize)) -> f32,
) -> (usize, usize) {
    let mut best = from;
    let mut best_score = f32::MIN;
    for cell in world.neighbors(from.0, from.1).into_iter().flatten() {
        // O ruído é menor do que 1, por isso nunca troca um passo melhor por um pior.
        let s = score(cell) + rng.range_f32(0.0, 0.5);
        if s > best_score {
            best_score = s;
            best = cell;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mundo() -> World {
        let mut rng = Rng::new(1);
        World::new(8, 8, VegetationParams::default(), &mut rng)
    }

    #[test]
    fn take_nao_retira_mais_do_que_existe() {
        let mut w = mundo();
        let tirado = w.take(0, 0, 1000.0);
        assert!(tirado <= w.capacity());
        assert_eq!(w.biomass_at(0, 0), 0.0);
    }

    #[test]
    fn celula_vazia_recupera_com_o_tempo() {
        let mut w = mundo();
        w.take(3, 3, 1000.0);
        for _ in 0..500 {
            w.grow();
        }
        assert!(w.biomass_at(3, 3) > 1.0);
    }

    #[test]
    fn biomassa_nunca_passa_a_capacidade() {
        let mut w = mundo();
        for _ in 0..2000 {
            w.grow();
        }
        for y in 0..w.height {
            for x in 0..w.width {
                assert!(w.biomass_at(x, y) <= w.capacity());
            }
        }
    }

    #[test]
    fn toca_mais_proxima() {
        let mut w = mundo();
        w.add_burrow(7, 7);
        w.add_burrow(1, 2);
        assert_eq!(w.nearest_burrow((0, 0), 5), Some((1, 2)));
        assert_eq!(w.nearest_burrow((0, 0), 2), None); // a (1, 2) fica a 3 passos
        assert_eq!(w.nearest_burrow((6, 6), 5), Some((7, 7)));
    }

    #[test]
    fn canto_tem_dois_vizinhos() {
        let w = mundo();
        let n = w.neighbors(0, 0).iter().flatten().count();
        assert_eq!(n, 2);
    }
}
