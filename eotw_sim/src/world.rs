//! O mundo: uma grelha de células, cada uma com uma quantidade de vegetação (biomassa).
//!
//! GDD 7.7: a vegetação é um recurso que se gasta e se regenera, não só decoração.

use serde::{Deserialize, Serialize};
use crate::resources::{Resource, ResourceKind, ResourceParams};
use crate::rng::Rng;
use crate::scent::ScentField;

/// A erva (relatório "Plantas a fundo", secção 4).
///
/// **Unidade:** 1 unidade de biomassa (UB) = 20 g de matéria seca. Uma célula
/// tem 100 m², por isso 1 g/m² = 5 UB por célula. É a unidade em que comem
/// todos os animais: um coelho ≈3,3 UB por dia (≈70 g, Cooke 2014), um veado
/// 100 (2 kg), um javali 40.
///
/// **Dois relógios.** As taxas por hora são reais (o crescimento, o que cada
/// animal come). Os stocks que passam de uma estação para a outra valem o
/// real a dividir por ≈10, porque o ano de jogo tem 36 dias e a primavera
/// tem 9: a erva não consegue acumular mais do que isso numa primavera.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VegetationParams {
    /// Biomassa máxima de uma célula, em UB: o que uma célula sem animais
    /// acumula num ano de jogo. A produção da dehesa (100–270 g/m²/ano) a
    /// dividir por ≈10 dá 10–27 g/m², ou 50–135 UB; a erva de pé num montado
    /// pastado é de 29–37 g/m² (Nair et al. 2019, verificado).
    pub capacity: f32,
    /// Crescimento médio por célula e por hora, em UB, com a célula vazia e um
    /// mês de multiplicador 1. Para 200 g/m²/ano: 200 × 5 / 8760 ≈ 0,11. A
    /// produção sobe com a chuva (Sala et al. 1988: −34 + 0,6 × mm).
    pub growth: f32,
}

impl Default for VegetationParams {
    fn default() -> Self {
        Self {
            capacity: 100.0,
            growth: 0.11,
        }
    }
}

/// Uma hora de crescimento de uma célula com `b` de erva, com o multiplicador
/// do mês `month` (ver `Climate::grass_month`). Cresce mais depressa vazia e
/// pára na capacidade. A mesma função serve a simulação completa e a região
/// agregada, para as duas não divergirem.
pub fn grass_step(b: f32, p: &VegetationParams, month: f32) -> f32 {
    (b + p.growth * month * (1.0 - b / p.capacity)).clamp(0.0, p.capacity)
}

/// Até quantas células à volta (em quadrado) uma árvore abriga os coelhos.
pub const TREE_COVER: usize = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    /// Árvores e pedras que o jogador pode recolher (GDD 7.7: entidades
    /// separadas da vegetação). São poucas, por isso uma lista chega.
    resources: Vec<Resource>,
    /// Células abrigadas por uma árvore com madeira (a `TREE_COVER` ou menos).
    /// Uma árvore cortada não abriga até voltar a ter madeira.
    cover: Vec<bool>,
    /// Clareiras: células à volta de uma árvore cortada que nenhuma árvore
    /// inteira abriga. Os coelhos sentem-se expostos e evitam-nas.
    clearing: Vec<bool>,
    /// A madeira que uma árvore precisa de ter para abrigar
    /// (`ResourceParams::shelter_wood`, posta em `regrow_resources`).
    shelter_wood: f32,
    /// Os campos de cheiro (GDD 7.6). A simulação deposita e espalha; os
    /// animais só os leem.
    pub scent: ScentField,
    params: VegetationParams,
    /// Células de rio (ver `river.rs`). Não têm erva, e os animais não
    /// entram nelas: `neighbors` não as dá.
    water: Vec<bool>,
    /// Quantas há, para não contar de novo.
    water_count: usize,
    /// Carcaças deixadas pelo lobo: (x, y, kg de carne). Só a raposa as come
    /// (os coelhos comem os restos da cabana, não carne).
    carcasses: Vec<(usize, usize, f32)>,
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
            resources: Vec::new(),
            cover: vec![false; width * height],
            clearing: vec![false; width * height],
            shelter_wood: ResourceParams::default().shelter_wood,
            scent: ScentField::new(width, height),
            params,
            water: vec![false; width * height],
            water_count: 0,
            carcasses: Vec::new(),
        }
    }

    pub fn add_carcass(&mut self, x: usize, y: usize, kg: f32) {
        match self.carcasses.iter_mut().find(|c| (c.0, c.1) == (x, y)) {
            Some(c) => c.2 += kg,
            None => self.carcasses.push((x, y, kg)),
        }
    }

    pub fn carcasses(&self) -> &[(usize, usize, f32)] {
        &self.carcasses
    }

    pub fn carcass_at(&self, x: usize, y: usize) -> f32 {
        self.carcasses.iter().find(|c| (c.0, c.1) == (x, y)).map_or(0.0, |c| c.2)
    }

    /// Come até `kg` da carcaça desta célula e devolve quanto comeu.
    pub fn take_carcass(&mut self, x: usize, y: usize, kg: f32) -> f32 {
        let Some(c) = self.carcasses.iter_mut().find(|c| (c.0, c.1) == (x, y)) else {
            return 0.0;
        };
        let taken = kg.min(c.2);
        c.2 -= taken;
        self.carcasses.retain(|c| c.2 > 0.01);
        taken
    }

    /// As carcaças apodrecem: perdem `fraction` por hora.
    pub fn rot_carcasses(&mut self, fraction: f32) {
        if self.carcasses.is_empty() {
            return;
        }
        for c in &mut self.carcasses {
            c.2 *= 1.0 - fraction;
        }
        self.carcasses.retain(|c| c.2 > 0.01);
    }

    /// Põe água numa célula: fica sem erva para sempre.
    pub fn add_water(&mut self, x: usize, y: usize) {
        let i = self.idx(x, y);
        if !self.water[i] {
            self.water[i] = true;
            self.water_count += 1;
        }
        self.biomass[i] = 0.0;
    }

    pub fn is_water(&self, x: usize, y: usize) -> bool {
        self.water[self.idx(x, y)]
    }

    /// As células de água, por ordem de linha e coluna.
    pub fn water_cells(&self) -> Vec<(usize, usize)> {
        (0..self.width * self.height)
            .filter(|&i| self.water[i])
            .map(|i| (i % self.width, i / self.width))
            .collect()
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
    /// Uma hora de crescimento de todas as células de terra, com o
    /// multiplicador do mês (ver `grass_step`).
    pub fn grow(&mut self, month: f32) {
        let p = &self.params;
        for (b, &water) in self.biomass.iter_mut().zip(&self.water) {
            if !water {
                *b = grass_step(*b, p, month);
            }
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

    /// Todas as tocas, por ordem de linha e coluna.
    pub fn burrows(&self) -> &[(usize, usize)] {
        &self.burrow_list
    }

    /// Põe a biomassa de uma célula num valor (limitado à capacidade).
    pub fn set_biomass(&mut self, x: usize, y: usize, value: f32) {
        let i = self.idx(x, y);
        self.biomass[i] = if self.water[i] { 0.0 } else { value.clamp(0.0, self.params.capacity) };
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

    /// Põe uma árvore ou uma pedra no terreno. Numa célula que já tenha um
    /// recurso não faz nada.
    pub fn add_resource(&mut self, r: Resource) {
        if self.resource_at(r.x, r.y).is_none() {
            self.resources.push(r);
            self.refresh_cover();
        }
    }

    /// A célula está abrigada por uma árvore (GDD 7.1: abrigo).
    pub fn tree_cover(&self, x: usize, y: usize) -> bool {
        self.cover[self.idx(x, y)]
    }

    /// As árvores voltam a crescer (ver `ResourceParams::regrow`), e as que
    /// voltam a ter madeira voltam a abrigar.
    /// As árvores só crescem, por isso o abrigo só muda quando uma árvore
    /// cortada volta a ter `shelter_wood`: só então se refaz (antes refazia-se
    /// a cada hora de jogo).
    pub fn regrow_resources(&mut self, params: &ResourceParams, days: f32, fruiting: bool) {
        let before = self.whole_trees();
        params.regrow(&mut self.resources, days, fruiting);
        let threshold_changed = self.shelter_wood != params.shelter_wood;
        self.shelter_wood = params.shelter_wood;
        if threshold_changed || self.whole_trees() != before {
            self.refresh_cover();
        }
    }

    fn whole_trees(&self) -> usize {
        self.resources
            .iter()
            .filter(|r| r.kind == ResourceKind::Tree && r.amount >= self.shelter_wood)
            .count()
    }

    /// A árvore tem madeira que chegue para abrigar (ver `refresh_cover`).
    pub fn is_whole_tree(&self, x: usize, y: usize) -> bool {
        self.resource_at(x, y)
            .is_some_and(|r| r.kind == ResourceKind::Tree && r.amount >= self.shelter_wood)
    }

    /// A célula é uma clareira: está ao lado de uma árvore cortada e nenhuma
    /// árvore inteira a abriga.
    pub fn clearing(&self, x: usize, y: usize) -> bool {
        self.clearing[self.idx(x, y)]
    }

    /// Volta a marcar as células abrigadas e as clareiras. São poucas
    /// árvores, por isso refazer tudo é barato.
    fn refresh_cover(&mut self) {
        self.cover.fill(false);
        self.clearing.fill(false);
        let (w, h) = (self.width, self.height);
        // As células a `TREE_COVER` ou menos de cada árvore, inteiras ou cortadas.
        let mut whole = Vec::new();
        let mut cut = Vec::new();
        for r in self.resources.iter().filter(|r| r.kind == ResourceKind::Tree) {
            let list = if r.amount >= self.shelter_wood { &mut whole } else { &mut cut };
            for y in r.y.saturating_sub(TREE_COVER)..=(r.y + TREE_COVER).min(h - 1) {
                for x in r.x.saturating_sub(TREE_COVER)..=(r.x + TREE_COVER).min(w - 1) {
                    list.push(y * w + x);
                }
            }
        }
        for i in whole {
            self.cover[i] = true;
        }
        for i in cut {
            self.clearing[i] = !self.cover[i];
        }
    }

    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }

    pub fn resource_at(&self, x: usize, y: usize) -> Option<&Resource> {
        self.resources.iter().find(|r| (r.x, r.y) == (x, y))
    }

    /// Retira até `amount` do recurso da célula e devolve o que foi realmente
    /// retirado (0 se não houver recurso). Uma árvore cortada fica no sítio,
    /// sem madeira, e volta a crescer.
    pub fn take_resource(&mut self, x: usize, y: usize, amount: f32) -> f32 {
        match self.resources.iter_mut().find(|r| (r.x, r.y) == (x, y)) {
            Some(r) => {
                let taken = amount.min(r.amount);
                r.amount -= taken;
                self.refresh_cover();
                taken
            }
            None => 0.0,
        }
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

    /// A vegetação média das células de terra (a água não conta).
    pub fn mean_biomass(&self) -> f32 {
        self.biomass.iter().sum::<f32>() / (self.biomass.len() - self.water_count) as f32
    }

    /// Quantas células são de terra.
    pub fn land_cells(&self) -> usize {
        self.width * self.height - self.water_count
    }

    /// As (até) 4 células vizinhas: cima, baixo, esquerda, direita.
    /// Devolve `None` nas posições que ficam fora do mapa.
    /// A água conta como fora do mapa: é por aqui que os animais não entram
    /// no rio.
    pub fn neighbors(&self, x: usize, y: usize) -> [Option<(usize, usize)>; 4] {
        let cells = [
            if y > 0 { Some((x, y - 1)) } else { None },
            if y + 1 < self.height { Some((x, y + 1)) } else { None },
            if x > 0 { Some((x - 1, y)) } else { None },
            if x + 1 < self.width { Some((x + 1, y)) } else { None },
        ];
        if self.water_count == 0 {
            return cells;
        }
        cells.map(|c| c.filter(|&(a, b)| !self.water[self.idx(a, b)]))
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

    /// Uma árvore inteira abriga as células à volta. Cortada, mesmo que só
    /// em parte, deixa uma clareira até voltar a estar inteira.
    #[test]
    fn a_arvore_abriga_ate_ser_cortada() {
        let mut w = mundo();
        let p = ResourceParams::default();
        w.add_resource(Resource { x: 3, y: 3, kind: ResourceKind::Tree, amount: p.tree_wood });
        w.add_resource(Resource { x: 7, y: 0, kind: ResourceKind::Rock, amount: p.rock_stone });
        assert!(w.tree_cover(3, 3) && w.tree_cover(2, 4) && w.tree_cover(4, 2));
        assert!(!w.tree_cover(5, 3), "a 2 células já não abriga");
        assert!(!w.tree_cover(7, 0), "as pedras não abrigam");
        assert!(!w.clearing(2, 4), "à volta de uma árvore inteira não há clareira");
        w.take_resource(3, 3, 1.0);
        assert!(!w.tree_cover(3, 3), "basta tirar-lhe 1 de madeira para não abrigar");
        assert!(w.clearing(2, 4) && w.clearing(3, 3), "fica uma clareira à volta");
        w.regrow_resources(&p, 0.5 / p.tree_regrowth, false);
        assert!(w.clearing(2, 4), "ainda não está inteira");
        w.regrow_resources(&p, 1.0 / p.tree_regrowth, false);
        assert!(w.tree_cover(3, 3) && !w.clearing(2, 4), "inteira, volta a abrigar");
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
            w.grow(1.0);
        }
        assert!(w.biomass_at(3, 3) > 1.0);
    }

    #[test]
    fn biomassa_nunca_passa_a_capacidade() {
        let mut w = mundo();
        for _ in 0..2000 {
            w.grow(1.0);
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
