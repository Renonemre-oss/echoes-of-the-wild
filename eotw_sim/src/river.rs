//! O rio da paisagem (GDD 18.2: "Uma pequena floresta com rio e clareira").
//!
//! Versão simplificada, sem pesquisa ainda: um rio de 2 células de largura
//! atravessa a paisagem de norte a sul, a serpentear, com vaus de tantas em
//! tantas células. A água não tem erva e os animais não entram nela (ver
//! `World::neighbors`): só atravessam nos vaus. O jogador passa a vau por
//! todo o lado, mas não constrói nem procura comida na água.

use serde::{Deserialize, Serialize};

use crate::rng::Rng;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RiverParams {
    /// Sem rio, a paisagem é igual à de antes.
    pub enabled: bool,
    /// Largura em células.
    pub width: usize,
    /// Um vau (uma linha sem água) a cada tantas células.
    pub ford_every: usize,
    /// Quanto se pode afastar, para cada lado, da linha a direito.
    pub meander: usize,
    /// Probabilidade, por linha, de o rio virar uma célula para um lado.
    pub turn_chance: f32,
    /// O rio fica pelo menos a esta distância do sítio onde o jogador começa.
    pub clear_of_player: usize,
}

impl Default for RiverParams {
    fn default() -> Self {
        Self { enabled: true, width: 2, ford_every: 12, meander: 4, turn_chance: 0.3, clear_of_player: 6 }
    }
}

impl RiverParams {
    /// As células de água de um mapa de `width` x `height`, em linha de
    /// norte a sul. `player_x` é a coluna onde o jogador começa.
    pub fn trace(&self, rng: &mut Rng, width: usize, height: usize, player_x: usize) -> Vec<(usize, usize)> {
        let room = self.meander + self.width;
        if !self.enabled || width < 2 * room + 2 * self.clear_of_player + 1 {
            return Vec::new();
        }
        // A coluna de partida: longe das margens e do jogador.
        let far = |x: usize| x.abs_diff(player_x) > self.clear_of_player + room;
        let options: Vec<usize> = (room..width - room).filter(|&x| far(x)).collect();
        if options.is_empty() {
            return Vec::new();
        }
        let centre = options[rng.range_usize(options.len())];
        let mut offset: i64 = 0;
        let limit = self.meander as i64;
        let mut cells = Vec::new();
        for y in 0..height {
            if rng.next_f32() < self.turn_chance {
                let turn = if rng.next_f32() < 0.5 { -1 } else { 1 };
                offset = (offset + turn).clamp(-limit, limit);
            }
            if self.ford_every > 0 && y % self.ford_every == self.ford_every / 2 {
                continue;
            }
            let x = (centre as i64 + offset) as usize;
            cells.extend((x..x + self.width).map(|x| (x, y)));
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_rio_atravessa_o_mapa_com_vaus() {
        let p = RiverParams::default();
        let cells = p.trace(&mut Rng::new(1), 120, 120, 12);
        let rows: std::collections::BTreeSet<usize> = cells.iter().map(|c| c.1).collect();
        assert_eq!(rows.len(), 120 - 120 / p.ford_every, "uma linha sem água a cada {}", p.ford_every);
        assert!(cells.iter().all(|c| c.0.abs_diff(12) > p.clear_of_player));
        // Sem buracos na diagonal: duas linhas seguidas com água tocam-se.
        for y in 1..120 {
            let row = |y: usize| cells.iter().filter(move |c| c.1 == y).map(|c| c.0).collect::<Vec<_>>();
            let (a, b) = (row(y - 1), row(y));
            if !a.is_empty() && !b.is_empty() {
                assert!(a.iter().any(|x| b.contains(x)), "linha {y}");
            }
        }
    }

    #[test]
    fn sem_rio_ou_mapa_pequeno_nao_ha_agua() {
        let off = RiverParams { enabled: false, ..RiverParams::default() };
        assert!(off.trace(&mut Rng::new(1), 120, 120, 12).is_empty());
        assert!(RiverParams::default().trace(&mut Rng::new(1), 24, 24, 12).is_empty());
    }
}
