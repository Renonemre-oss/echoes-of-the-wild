//! O ruído que o jogador faz a trabalhar (GDD 7.6: audição, "eventos de
//! ruído com posição, raio e intensidade").
//!
//! Versão simplificada, sem pesquisa ainda: só o jogador faz ruído, e só
//! enquanto trabalha. Durante essas horas o ruído alarga o alcance a que os
//! coelhos o ouvem (`RabbitParams::player_hearing`) e a distância a que as
//! raposas se afastam dele (`FoxParams::human_fear`). Uma célula são cerca
//! de 10 m: o machado ouve-se a uns 100 m. O ruído entre animais (o grito de
//! um coelho apanhado) fica para depois, porque mexe no equilíbrio medido.

use serde::{Deserialize, Serialize};

/// Até quantas células se ouve cada trabalho. 0 é silencioso.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NoiseParams {
    /// Cortar madeira e partir pedra.
    pub chop: usize,
    pub mine: usize,
    /// Construir o abrigo.
    pub build: usize,
    /// Fazer uma ferramenta.
    pub craft: usize,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self { chop: 10, mine: 10, build: 6, craft: 3 }
    }
}
