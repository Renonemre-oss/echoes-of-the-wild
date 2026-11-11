//! O dia e a noite (GDD 9.2 e 7.6).
//!
//! A luz vai de 1 (dia) a 0 (noite), com um amanhecer e um anoitecer em que
//! sobe e desce aos poucos. De noite os animais veem menos longe: o coelho
//! perde mais visão do que a raposa, por isso a raposa caça melhor de noite.
//! O olfato não muda, e de noite conta mais.

use serde::{Deserialize, Serialize};
use crate::sim::TICKS_PER_DAY;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaylightParams {
    /// A hora em que o amanhecer começa (a luz começa a subir).
    pub dawn: u64,
    /// A hora em que o anoitecer começa (a luz começa a descer).
    pub dusk: u64,
    /// Quantas horas dura o amanhecer, e o anoitecer.
    pub twilight: u64,
}

impl Default for DaylightParams {
    fn default() -> Self {
        Self {
            dawn: 5,
            dusk: 19,
            twilight: 2,
        }
    }
}

impl DaylightParams {
    /// A luz no fim da hora `hour` do dia (0 a 23): 1 de dia, 0 de noite.
    pub fn light(&self, hour: u64) -> f32 {
        let h = hour % TICKS_PER_DAY;
        let t = self.twilight as f32;
        if h < self.dawn || h >= self.dusk + self.twilight {
            0.0
        } else if h < self.dawn + self.twilight {
            (h - self.dawn) as f32 / t
        } else if h < self.dusk {
            1.0
        } else {
            1.0 - (h - self.dusk) as f32 / t
        }
    }
}

impl DaylightParams {
    /// O meio do dia: de três horas depois do nascer do sol (a meio do
    /// amanhecer) até três horas antes do pôr (a meio do anoitecer). É quando
    /// os coelhos bravos digerem na lura (cecotrofia, cerca de 08:00–17:00).
    pub fn midday(&self, hour: u64) -> bool {
        let h = hour % TICKS_PER_DAY;
        let half = self.twilight / 2;
        h >= self.dawn + half + 3 && h < self.dusk + half - 3
    }
}

/// A distância a que um animal vê com esta luz: `night` às escuras, `day`
/// com luz plena, e entre as duas no amanhecer e no anoitecer.
pub fn vision(day: usize, night: usize, light: f32) -> usize {
    night + ((day.saturating_sub(night)) as f32 * light).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_luz_sobe_de_manha_e_desce_a_tarde() {
        let d = DaylightParams::default();
        let luz: Vec<f32> = (0..24).map(|h| d.light(h)).collect();
        assert_eq!(luz[0], 0.0, "meia-noite");
        assert_eq!(luz[4], 0.0);
        assert_eq!(luz[5], 0.0, "o amanhecer começa às 5");
        assert_eq!(luz[6], 0.5);
        assert_eq!(luz[7], 1.0);
        assert_eq!(luz[12], 1.0, "meio-dia");
        assert_eq!(luz[19], 1.0, "o anoitecer começa às 19");
        assert_eq!(luz[20], 0.5);
        assert_eq!(luz[21], 0.0);
        assert_eq!(luz[23], 0.0);
        // Uma hora depois de 23 é meia-noite outra vez.
        assert_eq!(d.light(24), luz[0]);
    }

    #[test]
    fn o_meio_dia_vai_das_9_as_17() {
        let d = DaylightParams::default();
        let horas: Vec<u64> = (0..24).filter(|&h| d.midday(h)).collect();
        assert_eq!(horas, (9..17).collect::<Vec<_>>());
    }

    #[test]
    fn de_noite_ve_se_menos() {
        assert_eq!(vision(3, 2, 1.0), 3);
        assert_eq!(vision(3, 2, 0.0), 2);
        assert_eq!(vision(5, 4, 0.5), 5, "a meio do anoitecer arredonda para cima");
        assert_eq!(vision(5, 4, 0.4), 4);
    }
}
