//! Os recursos que o jogador recolhe e o inventário onde os leva (GDD 12,
//! Fase 1: "Recolher madeira e pedra"; GDD 19, etapa 6).
//!
//! As árvores e as pedras são entidades separadas da vegetação (GDD 7.7). Por
//! agora não mexem no ecossistema: os animais passam por elas, e cortar uma
//! árvore não muda a erva nem o abrigo. As árvores voltam a crescer devagar;
//! as pedras esgotam-se.

use std::collections::BTreeMap;

use crate::rng::Rng;

/// O que se leva no inventário.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Item {
    Wood,
    Stone,
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Wood => "madeira",
            Item::Stone => "pedra",
        }
    }
}

/// O tipo de recurso no terreno.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Tree,
    Rock,
}

impl ResourceKind {
    /// O que dá a quem o recolhe.
    pub fn item(self) -> Item {
        match self {
            ResourceKind::Tree => Item::Wood,
            ResourceKind::Rock => Item::Stone,
        }
    }
}

/// Uma árvore ou uma pedra numa célula, com o que ainda tem para dar.
#[derive(Clone, Debug, PartialEq)]
pub struct Resource {
    pub x: usize,
    pub y: usize,
    pub kind: ResourceKind,
    pub amount: f32,
}

#[derive(Clone, Debug)]
pub struct ResourceParams {
    /// Árvores e pedras em cada mapa de `REFERENCE_CELLS` células (24x24).
    pub trees: usize,
    pub rocks: usize,
    /// Madeira de uma árvore inteira, e pedra de uma pedra.
    pub tree_wood: f32,
    pub rock_stone: f32,
    /// Madeira que uma árvore volta a ganhar por dia, até `tree_wood`.
    pub tree_regrowth: f32,
    /// Horas de jogo que uma recolha gasta.
    pub gather_hours: u64,
    /// Quanto se recolhe de cada vez, sem ferramentas.
    pub gather_amount: f32,
    /// Quantas coisas cabem no inventário, de todos os tipos juntos.
    pub carry: u32,
}

impl Default for ResourceParams {
    fn default() -> Self {
        Self {
            trees: 20,
            rocks: 8,
            tree_wood: 4.0,
            rock_stone: 6.0,
            tree_regrowth: 0.5,
            gather_hours: 1,
            gather_amount: 1.0,
            carry: 20,
        }
    }
}

/// O tamanho do mapa (24x24) a que se referem `trees` e `rocks`.
const REFERENCE_CELLS: usize = 576;

impl ResourceParams {
    /// Espalha árvores e pedras num mapa de `width` x `height`, uma por célula
    /// no máximo e fora das células em que `free` diz que não. `rng` deve ser
    /// só para isto, para não mudar o resto do mundo da seed.
    pub fn place(
        &self,
        rng: &mut Rng,
        width: usize,
        height: usize,
        free: impl Fn((usize, usize)) -> bool,
    ) -> Vec<Resource> {
        let cells = width * height;
        let count = |n: usize| (n * cells).div_ceil(REFERENCE_CELLS);
        let mut taken = vec![false; cells];
        let mut out = Vec::new();
        for (kind, n, amount) in [
            (ResourceKind::Tree, count(self.trees), self.tree_wood),
            (ResourceKind::Rock, count(self.rocks), self.rock_stone),
        ] {
            // Algumas tentativas por recurso: num mapa cheio podem ficar menos.
            for _ in 0..n {
                for _ in 0..10 {
                    let (x, y) = (rng.range_usize(width), rng.range_usize(height));
                    if !taken[y * width + x] && free((x, y)) {
                        taken[y * width + x] = true;
                        out.push(Resource { x, y, kind, amount });
                        break;
                    }
                }
            }
        }
        out
    }

    /// Um dia de crescimento das árvores. As pedras não voltam.
    pub fn regrow(&self, resources: &mut [Resource], days: f32) {
        for r in resources.iter_mut().filter(|r| r.kind == ResourceKind::Tree) {
            r.amount = (r.amount + self.tree_regrowth * days).min(self.tree_wood);
        }
    }
}

/// O que o jogador leva consigo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inventory {
    items: BTreeMap<Item, u32>,
}

impl Inventory {
    pub fn count(&self, item: Item) -> u32 {
        self.items.get(&item).copied().unwrap_or(0)
    }

    pub fn total(&self) -> u32 {
        self.items.values().sum()
    }

    /// Junta até `n` coisas, sem passar de `carry` no total. Devolve quantas
    /// couberam.
    pub fn add(&mut self, item: Item, n: u32, carry: u32) -> u32 {
        let fits = n.min(carry.saturating_sub(self.total()));
        if fits > 0 {
            *self.items.entry(item).or_default() += fits;
        }
        fits
    }

    /// Tira `n` coisas, se houver; senão não tira nada.
    pub fn remove(&mut self, item: Item, n: u32) -> bool {
        let have = self.count(item);
        if have < n {
            return false;
        }
        if have == n {
            self.items.remove(&item);
        } else {
            self.items.insert(item, have - n);
        }
        true
    }

    /// O que há, por ordem fixa: madeira e depois pedra.
    pub fn iter(&self) -> impl Iterator<Item = (Item, u32)> + '_ {
        self.items.iter().map(|(&i, &n)| (i, n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_inventario_tem_limite() {
        let mut inv = Inventory::default();
        assert_eq!(inv.add(Item::Wood, 15, 20), 15);
        assert_eq!(inv.add(Item::Stone, 10, 20), 5, "só cabem mais 5");
        assert_eq!(inv.total(), 20);
        assert_eq!(inv.add(Item::Wood, 1, 20), 0);
        assert!(inv.remove(Item::Stone, 5));
        assert!(!inv.remove(Item::Stone, 1));
        assert_eq!(inv.count(Item::Stone), 0);
        assert_eq!(inv.iter().collect::<Vec<_>>(), vec![(Item::Wood, 15)]);
    }

    #[test]
    fn as_arvores_voltam_a_crescer_e_as_pedras_nao() {
        let p = ResourceParams::default();
        let mut r = vec![
            Resource { x: 0, y: 0, kind: ResourceKind::Tree, amount: 0.0 },
            Resource { x: 1, y: 0, kind: ResourceKind::Rock, amount: 0.0 },
        ];
        p.regrow(&mut r, 2.0);
        assert_eq!(r[0].amount, 2.0 * p.tree_regrowth);
        assert_eq!(r[1].amount, 0.0);
        p.regrow(&mut r, 100.0);
        assert_eq!(r[0].amount, p.tree_wood, "não passa de uma árvore inteira");
    }

    #[test]
    fn os_recursos_espalham_se_uma_por_celula() {
        let p = ResourceParams::default();
        let r = p.place(&mut Rng::new(1), 24, 24, |(x, _)| x != 0);
        assert_eq!(r.iter().filter(|r| r.kind == ResourceKind::Tree).count(), p.trees);
        assert_eq!(r.iter().filter(|r| r.kind == ResourceKind::Rock).count(), p.rocks);
        assert!(r.iter().all(|r| r.x != 0), "fora das células proibidas");
        let mut cells: Vec<_> = r.iter().map(|r| (r.x, r.y)).collect();
        cells.sort();
        cells.dedup();
        assert_eq!(cells.len(), r.len());
        // Num mapa maior, mais recursos.
        assert_eq!(p.place(&mut Rng::new(1), 48, 48, |_| true).len(), 4 * r.len());
    }
}
