//! O abrigo que o jogador constrói (GDD 10.2 e 12, Fase 1).
//!
//! O jogador começa sem abrigo. O primeiro objetivo é fazer um, e depois
//! melhorá-lo no mesmo sítio, uma etapa de cada vez. As quatro etapas seguem
//! os quatro tipos de chozo de pastor da Estremadura (Martín Galindo 2006):
//! todo de plantas, base de pedra, e por fim pedra com lareira. Em vez da
//! cúpula toda de pedra, a primeira etapa é o abrigo de ramos encostado a uma
//! árvore do manual de sobrevivência FM 21-76. Por todo o mundo, a passagem
//! do provisório ao permanente faz-se assim: no mesmo sítio, com a mesma
//! planta, trocando de material (Star Carr, Howick, os chozos). As notas
//! estão em `research_notes/Abrigos construídos pelo jogador/`.
//!
//! Por agora o abrigo ainda não protege o jogador de nada, porque o jogador
//! ainda não tem frio nem cansaço. O que já faz é cheirar a gente: os
//! animais afastam-se do acampamento, e mais quando há lareira (fumo).

use serde::{Deserialize, Serialize};
use crate::resources::Item;

/// As etapas, pela ordem em que se constroem.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Um abrigo de ramos (lean-to): uma trave entre duas árvores e 5–8 varas
    /// encostadas (FM 21-76). Corta o vento de um lado.
    Ramos,
    /// Os ramos cobertos de folhas e uma cama de folhas por dentro. O manual
    /// pede pelo menos 1 m de folhas por cima e 30 cm de cama, porque em
    /// repouso se perde muito calor para o chão.
    Folhas,
    /// Uma base de pedra com o cone de ramos e folhas por cima: o segundo
    /// tipo de chozo. Protege do vento por todos os lados.
    BasePedra,
    /// Pedra com lareira dentro, como o último tipo de chozo, que já tinha
    /// chaminé. A cúpula de pedra tinha uma laje com um buraco de 20 cm para
    /// o fumo.
    Lareira,
}

impl Stage {
    pub const ALL: [Stage; 4] = [Stage::Ramos, Stage::Folhas, Stage::BasePedra, Stage::Lareira];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Ramos => "abrigo de ramos",
            Stage::Folhas => "cabana de folhas",
            Stage::BasePedra => "chozo com base de pedra",
            Stage::Lareira => "chozo com lareira",
        }
    }

    /// O nome com o artigo, para as frases: "a cabana de folhas".
    pub fn with_article(self) -> String {
        let article = if self == Stage::Folhas { "a" } else { "o" };
        format!("{article} {}", self.name())
    }

    pub fn next(self) -> Option<Stage> {
        Stage::ALL.get(self as usize + 1).copied()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShelterParams {
    /// O que cada etapa custa, por ordem de `Stage::ALL`. São decisões de
    /// jogo, comprimidas: o manual pede 5–8 varas para o abrigo de ramos, e
    /// uma árvore inteira dá 4 de madeira, por isso 6 de madeira já obriga
    /// a cortar duas árvores.
    pub cost: [Vec<(Item, u32)>; 4],
    /// Horas de jogo que cada etapa demora. Uma pessoa sozinha faz uma
    /// cabana leve de ramos e folhas em poucas horas (Baka, Mbuti, San,
    /// Apache, Nenets), e um acampamento de wickiups levou quase um dia de
    /// trabalho (Morgan et al. 2018, só o resumo). A pedra demora mais.
    pub hours: [u64; 4],
    /// Cheiro humano que o abrigo deixa por hora, por etapa. O jogador deixa
    /// 5 (`ScentParams::human_per_tick`). Um abrigo vazio cheira menos do que
    /// o jogador, e a lareira cheira mais, por causa do fumo.
    pub scent_per_tick: [f32; 4],
    /// Vegetação gasta por cada folha recolhida. A erva da célula só dá
    /// folhas se tiver pelo menos isto.
    pub leaf_biomass: f32,
}

impl Default for ShelterParams {
    fn default() -> Self {
        Self {
            cost: [
                vec![(Item::Wood, 6)],
                vec![(Item::Leaves, 10)],
                vec![(Item::Stone, 8), (Item::Wood, 6)],
                vec![(Item::Stone, 6)],
            ],
            hours: [2, 3, 6, 4],
            scent_per_tick: [1.0, 1.5, 2.0, 6.0],
            leaf_biomass: 2.0,
        }
    }
}

impl ShelterParams {
    pub fn cost_of(&self, stage: Stage) -> &[(Item, u32)] {
        &self.cost[stage as usize]
    }

    pub fn hours_of(&self, stage: Stage) -> u64 {
        self.hours[stage as usize]
    }

    pub fn scent_of(&self, stage: Stage) -> f32 {
        self.scent_per_tick[stage as usize]
    }
}

/// O abrigo, numa célula da paisagem inteira.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shelter {
    pub x: usize,
    pub y: usize,
    pub stage: Stage,
}

/// O resultado de uma tentativa de construir (`Landscape::build`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    /// Construiu esta etapa. Passaram as horas dela.
    Built(Stage),
    /// Falta tanto deste material para a etapa seguinte.
    Missing(Stage, Item, u32),
    /// O primeiro abrigo encosta-se a uma árvore: tem de haver uma ao lado.
    NeedsTree,
    /// Não se constrói em cima de uma árvore, de uma pedra ou de uma toca.
    Blocked,
    /// Já há um abrigo noutro sítio: as melhorias fazem-se nele.
    Elsewhere,
    /// O abrigo já está na última etapa.
    Finished,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_etapas_seguem_por_ordem() {
        let mut s = Stage::Ramos;
        let mut nomes = vec![s.name()];
        while let Some(n) = s.next() {
            s = n;
            nomes.push(s.name());
        }
        assert_eq!(
            nomes,
            ["abrigo de ramos", "cabana de folhas", "chozo com base de pedra", "chozo com lareira"]
        );
    }

    #[test]
    fn a_lareira_cheira_mais_do_que_o_jogador() {
        let p = ShelterParams::default();
        let jogador = crate::scent::ScentParams::default().human_per_tick;
        assert!(p.scent_of(Stage::Ramos) < jogador);
        assert!(p.scent_of(Stage::Lareira) > jogador);
    }
}
