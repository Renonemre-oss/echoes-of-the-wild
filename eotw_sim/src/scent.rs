//! Campos de cheiro (GDD 7.6): uma grelha por tipo de cheiro, com um valor por célula.
//!
//! As fontes (restos de comida, coelhos) depositam cheiro na sua célula. A cada
//! tick o cheiro espalha-se um pouco para as células vizinhas e decai. Os
//! animais não sabem onde está a comida: só cheiram a sua célula e as 4 à
//! volta, e sobem o gradiente do tipo que lhes interessa.

use crate::rng::Rng;
use crate::world::World;

/// Os tipos de cheiro que já existem. Carcaça e predador vêm mais tarde.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scent {
    /// Restos de comida deixados pelo jogador.
    Food = 0,
    /// Coelhos (presas da raposa).
    Prey = 1,
    /// O jogador: fica no caminho por onde passou, e as raposas evitam-no.
    Human = 2,
}

const KINDS: usize = 3;

#[derive(Clone, Debug)]
pub struct ScentParams {
    /// Fração do cheiro de uma célula que se troca com as vizinhas em cada tick.
    pub diffusion: f32,
    /// Fração do cheiro que desaparece em cada tick.
    pub decay: f32,
    /// Cheiro depositado por tick por cada unidade de restos de comida.
    pub food_per_scrap: f32,
    /// Cheiro depositado por tick por cada coelho fora da toca. Dentro da toca
    /// o cheiro fica debaixo da terra: assim a toca continua a ser um refúgio.
    pub prey_per_rabbit: f32,
    /// Cheiro depositado por tick pelo jogador.
    pub human_per_tick: f32,
}

impl Default for ScentParams {
    fn default() -> Self {
        Self {
            diffusion: 0.6,
            decay: 0.01,
            food_per_scrap: 1.0,
            prey_per_rabbit: 1.0,
            human_per_tick: 5.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScentField {
    width: usize,
    height: usize,
    layers: [Vec<f32>; KINDS],
    /// Grelha auxiliar para o passo de difusão (evita alocar em cada tick).
    scratch: Vec<f32>,
}

impl ScentField {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            layers: std::array::from_fn(|_| vec![0.0; width * height]),
            scratch: vec![0.0; width * height],
        }
    }

    pub fn at(&self, kind: Scent, x: usize, y: usize) -> f32 {
        self.layers[kind as usize][y * self.width + x]
    }

    pub fn deposit(&mut self, kind: Scent, x: usize, y: usize, amount: f32) {
        self.layers[kind as usize][y * self.width + x] += amount;
    }

    /// Um tick: cada célula troca uma parte do cheiro com a média das vizinhas,
    /// e depois tudo decai. Nas margens só contam as vizinhas que existem, por
    /// isso o cheiro não "sai" do mapa.
    pub fn spread(&mut self, p: &ScentParams) {
        let (w, h) = (self.width, self.height);
        for layer in &mut self.layers {
            for y in 0..h {
                for x in 0..w {
                    let mut sum = 0.0;
                    let mut n = 0.0;
                    if y > 0 {
                        sum += layer[(y - 1) * w + x];
                        n += 1.0;
                    }
                    if y + 1 < h {
                        sum += layer[(y + 1) * w + x];
                        n += 1.0;
                    }
                    if x > 0 {
                        sum += layer[y * w + x - 1];
                        n += 1.0;
                    }
                    if x + 1 < w {
                        sum += layer[y * w + x + 1];
                        n += 1.0;
                    }
                    let here = layer[y * w + x];
                    let mixed = here + p.diffusion * (sum / n - here);
                    self.scratch[y * w + x] = mixed * (1.0 - p.decay);
                }
            }
            std::mem::swap(layer, &mut self.scratch);
        }
    }
}

/// A célula vizinha onde o cheiro `kind` é mais forte, se for mais forte do que
/// aqui e acima de `min_scent` (a sensibilidade do nariz). `None` quando não há
/// gradiente que se sinta: o animal tem de usar outro sentido ou deambular.
pub fn sniff(
    world: &World,
    rng: &mut Rng,
    kind: Scent,
    from: (usize, usize),
    min_scent: f32,
) -> Option<(usize, usize)> {
    let field = &world.scent;
    let here = field.at(kind, from.0, from.1);
    let neighbors = world.neighbors(from.0, from.1);
    // Nada que se sinta: sai logo, sem gastar números do `Rng`. Assim um mundo
    // sem cabana não é alterado por um cheiro que não existe.
    let strongest = neighbors
        .iter()
        .flatten()
        .map(|&(x, y)| field.at(kind, x, y))
        .fold(here, f32::max);
    if strongest < min_scent {
        return None;
    }
    let mut best = None;
    let mut best_value = here;
    for (x, y) in neighbors.into_iter().flatten() {
        // Um pouco de ruído desempata células iguais sem trocar uma melhor por uma pior.
        let v = field.at(kind, x, y) * (1.0 + rng.range_f32(0.0, 0.001));
        if v > best_value {
            best_value = v;
            best = Some((x, y));
        }
    }
    if best_value < min_scent {
        return None;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::VegetationParams;

    #[test]
    fn o_cheiro_espalha_se_e_decai() {
        let p = ScentParams::default();
        let mut f = ScentField::new(9, 9);
        f.deposit(Scent::Food, 4, 4, 100.0);
        f.spread(&p);
        assert!(f.at(Scent::Food, 4, 5) > 0.0, "devia ter chegado à vizinha");
        assert!(f.at(Scent::Food, 4, 4) < 100.0);
        assert_eq!(f.at(Scent::Prey, 4, 4), 0.0, "os tipos não se misturam");
        // Sem fonte, acaba por desaparecer.
        for _ in 0..2000 {
            f.spread(&p);
        }
        let total: f32 = (0..9).flat_map(|y| (0..9).map(move |x| (x, y))).map(|(x, y)| f.at(Scent::Food, x, y)).sum();
        assert!(total < 0.01, "ainda há {total}");
    }

    #[test]
    fn subir_o_gradiente_leva_a_fonte() {
        let p = ScentParams::default();
        let mut rng = Rng::new(1);
        let mut world = World::new(16, 16, VegetationParams::default(), &mut rng);
        for _ in 0..200 {
            world.scent.deposit(Scent::Food, 12, 3, 10.0);
            world.scent.spread(&p);
        }
        let mut pos = (2, 13);
        for _ in 0..30 {
            match sniff(&world, &mut rng, Scent::Food, pos, 0.0) {
                Some(next) => pos = next,
                None => break,
            }
        }
        assert_eq!(pos, (12, 3));
    }

    #[test]
    fn longe_demais_nao_se_sente() {
        let mut rng = Rng::new(1);
        let mut world = World::new(16, 16, VegetationParams::default(), &mut rng);
        world.scent.deposit(Scent::Food, 12, 3, 1.0);
        world.scent.spread(&ScentParams::default());
        assert_eq!(sniff(&world, &mut rng, Scent::Food, (2, 13), 0.5), None);
    }
}
