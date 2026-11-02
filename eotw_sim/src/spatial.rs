//! Grelha espacial: divide o mapa em quadrados de `BUCKET` células e guarda,
//! em cada um, os índices dos animais que lá estão.
//!
//! Serve para um animal procurar só à sua volta em vez de percorrer a lista
//! inteira. Sem isto, num mapa de 192x192 cada raposa via os 20 000 coelhos e
//! cada coelho as 200 raposas em cada tick, e 90% do tempo ia nisso.
//!
//! `near` devolve os índices por ordem crescente, a mesma ordem da lista
//! original. Assim os empates resolvem-se como antes e a simulação dá
//! exatamente os mesmos resultados, só mais depressa.

/// Lado de cada quadrado da grelha, em células. Perto do alcance da vista
/// dos animais (3 a 9), para uma procura tocar em poucos quadrados.
const BUCKET: usize = 8;

#[derive(Clone, Debug, Default)]
pub struct Grid {
    cols: usize,
    rows: usize,
    buckets: Vec<Vec<usize>>,
}

impl Grid {
    /// Uma grelha para um mapa `width` x `height`, com estas posições
    /// (o índice de cada posição é o índice do animal na sua lista).
    pub fn build(
        width: usize,
        height: usize,
        positions: impl Iterator<Item = (usize, usize)>,
    ) -> Self {
        let mut grid = Self::default();
        grid.rebuild(width, height, positions);
        grid
    }

    /// Como `build`, mas reaproveita a memória da grelha anterior. A simulação
    /// refaz as grelhas em cada tick; alocar de novo tornava os mapas
    /// pequenos duas vezes mais lentos do que sem grelha.
    pub fn rebuild(
        &mut self,
        width: usize,
        height: usize,
        positions: impl Iterator<Item = (usize, usize)>,
    ) {
        self.cols = width.div_ceil(BUCKET);
        self.rows = height.div_ceil(BUCKET);
        self.buckets.resize_with(self.cols * self.rows, Vec::new);
        for b in &mut self.buckets {
            b.clear();
        }
        for (i, (x, y)) in positions.enumerate() {
            self.buckets[(y / BUCKET) * self.cols + x / BUCKET].push(i);
        }
    }

    /// Os índices de todos os animais cujo quadrado toca no quadrado de lado
    /// `2 * radius + 1` centrado em `at`, por ordem crescente. Pode incluir
    /// alguns que estão mais longe do que `radius`: quem chama ainda tem de
    /// verificar a distância. Usa `out` para não alocar em cada procura.
    pub fn near(&self, at: (usize, usize), radius: usize, out: &mut Vec<usize>) {
        out.clear();
        let bx0 = at.0.saturating_sub(radius) / BUCKET;
        let by0 = at.1.saturating_sub(radius) / BUCKET;
        let bx1 = ((at.0 + radius) / BUCKET).min(self.cols - 1);
        let by1 = ((at.1 + radius) / BUCKET).min(self.rows - 1);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                out.extend_from_slice(&self.buckets[by * self.cols + bx]);
            }
        }
        out.sort_unstable();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::world::distance;

    /// A grelha encontra exatamente os mesmos animais que uma procura na lista
    /// inteira, pela mesma ordem.
    #[test]
    fn a_grelha_encontra_o_mesmo_que_a_lista() {
        let mut rng = Rng::new(3);
        let (w, h) = (50, 37);
        let pos: Vec<(usize, usize)> = (0..400)
            .map(|_| (rng.range_usize(w), rng.range_usize(h)))
            .collect();
        let grid = Grid::build(w, h, pos.iter().copied());
        let mut out = Vec::new();
        for _ in 0..200 {
            let at = (rng.range_usize(w), rng.range_usize(h));
            let radius = rng.range_usize(12);
            grid.near(at, radius, &mut out);
            let pela_grelha: Vec<usize> =
                out.iter().copied().filter(|&i| distance(pos[i], at) <= radius).collect();
            let pela_lista: Vec<usize> =
                (0..pos.len()).filter(|&i| distance(pos[i], at) <= radius).collect();
            assert_eq!(pela_grelha, pela_lista, "em {at:?} com raio {radius}");
        }
    }
}
