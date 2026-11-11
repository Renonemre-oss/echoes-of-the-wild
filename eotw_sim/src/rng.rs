//! Gerador de números pseudoaleatórios determinístico (algoritmo SplitMix64).
//!
//! Porquê não usar uma biblioteca? Para não ter dependências e porque queremos
//! controlar exatamente a sequência: a mesma seed tem de dar sempre o mesmo mundo.

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Um gerador próprio para uma combinação de números (por exemplo seed,
    /// região, tick e espécie). Cada combinação dá uma sequência diferente e
    /// sempre a mesma, e mudar uma parte da simulação (quantas raposas há,
    /// por exemplo) não mexe nos números que as outras partes recebem.
    pub fn derived(parts: &[u64]) -> Self {
        let mut rng = Self::new(0x5EED);
        for &part in parts {
            rng.state ^= part;
            rng.state = rng.next_u64();
        }
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Número entre 0.0 (incluído) e 1.0 (excluído).
    pub fn next_f32(&mut self) -> f32 {
        // Usamos 24 bits porque é o que um f32 consegue representar sem perdas.
        ((self.next_u64() >> 40) as f32) / ((1u64 << 24) as f32)
    }

    /// Número entre `min` e `max`.
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    /// Índice entre 0 e `n - 1`. `n` tem de ser maior que zero.
    pub fn range_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesma_seed_da_mesma_sequencia() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn seeds_diferentes_dao_sequencias_diferentes() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn derivados_dependem_de_todas_as_partes() {
        let a = Rng::derived(&[1, 2, 3]).next_u64();
        assert_eq!(a, Rng::derived(&[1, 2, 3]).next_u64());
        assert_ne!(a, Rng::derived(&[1, 2, 4]).next_u64());
        assert_ne!(a, Rng::derived(&[2, 1, 3]).next_u64());
        assert_ne!(Rng::derived(&[0]).next_u64(), Rng::derived(&[0, 0]).next_u64());
    }

    #[test]
    fn next_f32_fica_entre_zero_e_um() {
        let mut r = Rng::new(3);
        for _ in 0..1000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
