//! Os recursos que o jogador recolhe, o inventário onde os leva e as
//! ferramentas que faz com eles (GDD 12, Fase 1: "Recolher madeira e pedra"
//! e "Criar ferramentas básicas"; GDD 19, etapa 6).
//!
//! As árvores e as pedras são entidades separadas da vegetação (GDD 7.7). Por
//! agora não mexem no ecossistema: os animais passam por elas, e cortar uma
//! árvore não muda a erva nem o abrigo. As árvores voltam a crescer devagar;
//! as pedras esgotam-se.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::rng::Rng;

/// O que se leva no inventário.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Item {
    Wood,
    Stone,
    /// Folhas e erva seca, para cobrir o abrigo e fazer a cama. Recolhem-se
    /// da erva, onde não há árvore nem pedra.
    Leaves,
    /// Ervas, raízes e cogumelos que se comem (ver `survival.rs`). Procuram-se
    /// na erva, com a tecla F.
    Food,
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Wood => "madeira",
            Item::Stone => "pedra",
            Item::Leaves => "folhas",
            Item::Food => "comida",
        }
    }
}

/// Uma ferramenta (GDD 10.4: ferramentas de recolha). Recolhe o dobro do
/// recurso a que se destina, e parte-se ao fim de `tool_uses` recolhas.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool {
    Axe,
    Pickaxe,
}

impl Tool {
    pub const ALL: [Tool; 2] = [Tool::Axe, Tool::Pickaxe];

    pub fn name(self) -> &'static str {
        match self {
            Tool::Axe => "machado",
            Tool::Pickaxe => "picareta",
        }
    }

    /// O nome com o artigo: "um machado", "uma picareta".
    pub fn with_article(self) -> &'static str {
        match self {
            Tool::Axe => "um machado",
            Tool::Pickaxe => "uma picareta",
        }
    }

    /// O que custa fazer: madeira e pedra (GDD 10.5: poucas quantidades).
    pub fn recipe(self) -> [(Item, u32); 2] {
        match self {
            Tool::Axe => [(Item::Wood, 3), (Item::Stone, 2)],
            Tool::Pickaxe => [(Item::Wood, 2), (Item::Stone, 3)],
        }
    }

    /// A ferramenta que recolhe melhor este material (nenhuma para as folhas).
    pub fn for_item(item: Item) -> Option<Tool> {
        match item {
            Item::Wood => Some(Tool::Axe),
            Item::Stone => Some(Tool::Pickaxe),
            Item::Leaves | Item::Food => None,
        }
    }
}

/// O tipo de recurso no terreno.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Resource {
    pub x: usize,
    pub y: usize,
    pub kind: ResourceKind,
    pub amount: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceParams {
    /// Árvores e pedras em cada mapa de `REFERENCE_CELLS` células (24x24).
    pub trees: usize,
    pub rocks: usize,
    /// Madeira de uma árvore inteira, e pedra de uma pedra.
    pub tree_wood: f32,
    pub rock_stone: f32,
    /// Madeira que uma árvore volta a ganhar por dia, até `tree_wood`. Com
    /// 0,1, uma árvore cortada leva 40 dias a ficar inteira: cortar tem
    /// consequências que duram.
    pub tree_regrowth: f32,
    /// Madeira que uma árvore precisa de ter para abrigar os coelhos. Com
    /// `tree_wood`, só abriga inteira: à volta de uma árvore cortada, mesmo
    /// que só em parte, fica uma clareira até ela voltar a crescer.
    pub shelter_wood: f32,
    /// Horas de jogo que uma recolha gasta.
    pub gather_hours: u64,
    /// Quanto se recolhe de cada vez, sem ferramentas.
    pub gather_amount: f32,
    /// Quantas coisas cabem no inventário, de todos os tipos juntos. As
    /// ferramentas não contam.
    pub carry: u32,
    /// Quanto se recolhe de cada vez com a ferramenta certa.
    pub tool_amount: f32,
    /// Quantas recolhas aguenta uma ferramenta antes de se partir.
    pub tool_uses: u32,
    /// Horas de jogo que fazer uma ferramenta gasta.
    pub craft_hours: u64,
}

impl Default for ResourceParams {
    fn default() -> Self {
        Self {
            trees: 20,
            rocks: 8,
            tree_wood: 4.0,
            rock_stone: 6.0,
            tree_regrowth: 0.1,
            shelter_wood: 4.0,
            gather_hours: 1,
            gather_amount: 1.0,
            carry: 20,
            tool_amount: 2.0,
            tool_uses: 30,
            craft_hours: 1,
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
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Inventory {
    items: BTreeMap<Item, u32>,
    /// As ferramentas, no máximo uma de cada, com as recolhas que ainda aguentam.
    tools: BTreeMap<Tool, u32>,
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

    /// O que há, por ordem fixa: madeira, pedra e folhas.
    pub fn iter(&self) -> impl Iterator<Item = (Item, u32)> + '_ {
        self.items.iter().map(|(&i, &n)| (i, n))
    }

    /// As recolhas que a ferramenta ainda aguenta (`None` se não a houver).
    pub fn tool(&self, tool: Tool) -> Option<u32> {
        self.tools.get(&tool).copied()
    }

    /// As ferramentas que há, por ordem fixa, com as recolhas que aguentam.
    pub fn tools(&self) -> impl Iterator<Item = (Tool, u32)> + '_ {
        self.tools.iter().map(|(&t, &n)| (t, n))
    }

    /// Faz a ferramenta, se houver os materiais e ainda não a houver.
    pub fn craft(&mut self, tool: Tool, uses: u32) -> Craft {
        if self.tools.contains_key(&tool) {
            return Craft::AlreadyHave;
        }
        for (item, n) in tool.recipe() {
            if self.count(item) < n {
                return Craft::Missing(item, n - self.count(item));
            }
        }
        for (item, n) in tool.recipe() {
            self.remove(item, n);
        }
        self.tools.insert(tool, uses);
        Craft::Made
    }

    /// Gasta uma recolha da ferramenta. Devolve `true` se ela se partiu.
    pub fn wear(&mut self, tool: Tool) -> bool {
        let Some(left) = self.tools.get_mut(&tool) else {
            return false;
        };
        *left = left.saturating_sub(1);
        if *left == 0 {
            self.tools.remove(&tool);
            return true;
        }
        false
    }
}

/// O resultado de tentar fazer uma ferramenta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Craft {
    Made,
    /// Faltam tantas unidades deste material.
    Missing(Item, u32),
    /// Já há uma: só se leva uma de cada.
    AlreadyHave,
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
    fn fazer_um_machado_gasta_madeira_e_pedra() {
        let mut inv = Inventory::default();
        inv.add(Item::Wood, 4, 20);
        inv.add(Item::Stone, 1, 20);
        assert_eq!(inv.craft(Tool::Axe, 30), Craft::Missing(Item::Stone, 1));
        assert_eq!(inv.total(), 5, "sem materiais não gasta nada");
        inv.add(Item::Stone, 1, 20);
        assert_eq!(inv.craft(Tool::Axe, 30), Craft::Made);
        assert_eq!((inv.count(Item::Wood), inv.count(Item::Stone)), (1, 0));
        assert_eq!(inv.tool(Tool::Axe), Some(30));
        inv.add(Item::Wood, 5, 20);
        inv.add(Item::Stone, 5, 20);
        assert_eq!(inv.craft(Tool::Axe, 30), Craft::AlreadyHave);
    }

    #[test]
    fn a_ferramenta_parte_se_ao_fim_dos_usos() {
        let mut inv = Inventory::default();
        inv.add(Item::Wood, 2, 20);
        inv.add(Item::Stone, 3, 20);
        assert_eq!(inv.craft(Tool::Pickaxe, 2), Craft::Made);
        assert!(!inv.wear(Tool::Pickaxe));
        assert_eq!(inv.tool(Tool::Pickaxe), Some(1));
        assert!(inv.wear(Tool::Pickaxe), "à segunda parte-se");
        assert_eq!(inv.tool(Tool::Pickaxe), None);
        assert!(!inv.wear(Tool::Pickaxe), "sem ferramenta não há nada para gastar");
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
