//! O coelho: o primeiro animal da simulação.
//!
//! Tem uma necessidade (energia), procura vegetação, come e reproduz-se quando está
//! bem alimentado. Se vê uma raposa, foge para a toca mais próxima ou, se não houver
//! nenhuma perto, para longe da raposa. A fuga tem prioridade sobre a fome, exceto
//! quando o coelho está a morrer de fome: aí arrisca e vai comer (GDD 17.3).
//! Com fome e sem comida onde está, vai para a célula vizinha com mais
//! vegetação; se nenhuma tiver que chegue, segue o cheiro de restos de comida.

use crate::rng::Rng;
use crate::scent::{sniff, Scent};
use crate::world::{distance, step_away, step_toward, World};

#[derive(Clone, Debug)]
pub struct RabbitParams {
    pub energy_max: f32,
    pub energy_start: f32,
    /// Acima deste valor o coelho descansa em vez de procurar comida.
    pub full_energy: f32,
    /// Gasto de energia por tick, mesmo parado.
    pub metabolism: f32,
    /// Gasto extra de energia ao mudar de célula.
    pub move_cost: f32,
    /// Biomassa máxima que come por tick (quando há muita comida).
    pub bite: f32,
    /// Quanta biomassa tem de haver numa célula para o coelho comer a meio da
    /// sua capacidade. Quanto maior, mais difícil é comer em células pobres.
    /// Isto impede que os coelhos pastem tudo até ao chão (GDD 7.8).
    pub half_saturation: f32,
    /// Biomassa mínima numa célula para valer a pena comer lá. Se for demasiado
    /// baixa, os coelhos ficam presos em células pobres em vez de procurarem melhores
    /// (com 0.5 a população extinguia-se: ver DIARIO.md).
    pub min_food: f32,
    pub energy_per_biomass: f32,
    pub repro_threshold: f32,
    pub repro_cost: f32,
    pub newborn_energy: f32,
    pub repro_cooldown: u64,
    pub maturity_ticks: u64,
    pub max_age_ticks: u64,
    /// Até quantas células de distância vê uma raposa. A raposa vê mais longe,
    /// por isso às vezes chega perto antes de o coelho dar por ela.
    pub vision: usize,
    /// Abaixo desta energia o coelho deixa de fugir e arrisca ir comer. Sem isto,
    /// com raposas por perto, morriam mais coelhos de fome (escondidos, com a
    /// vegetação cheia à volta) do que caçados. Tem de ficar abaixo de
    /// `newborn_energy`.
    pub desperate_energy: f32,
    /// O cheiro de comida mais fraco que o coelho sente (GDD 7.6: olfato médio).
    /// Com os valores atuais de `ScentParams`, chega a cerca de 7 células da cabana.
    pub smell_min: f32,
}

impl Default for RabbitParams {
    fn default() -> Self {
        Self {
            energy_max: 100.0,
            energy_start: 60.0,
            full_energy: 90.0,
            metabolism: 1.0,
            move_cost: 0.3,
            bite: 0.3,
            half_saturation: 3.0,
            min_food: 1.5,
            energy_per_biomass: 8.0,
            repro_threshold: 80.0,
            repro_cost: 40.0,
            // Acima de `desperate_energy`, para as crias fugirem desde que nascem.
            // Com 30 (igual ao limiar) nasciam já "desesperadas" e não fugiam.
            newborn_energy: 35.0,
            repro_cooldown: 24 * 5,
            maturity_ticks: 24 * 10,
            max_age_ticks: 24 * 120,
            vision: 3,
            desperate_energy: 30.0,
            smell_min: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Rabbit {
    pub x: usize,
    pub y: usize,
    pub energy: f32,
    pub age: u64,
    pub cooldown: u64,
    /// Foi caçado por uma raposa neste tick (a simulação retira-o no fim do tick).
    pub eaten: bool,
}

impl Rabbit {
    pub fn newborn(x: usize, y: usize, energy: f32) -> Self {
        Self {
            x,
            y,
            energy,
            age: 0,
            cooldown: 0,
            eaten: false,
        }
    }

    /// Um tick de vida. `foxes` são as posições das raposas.
    /// Devolve uma cria se o coelho se reproduziu neste tick.
    pub fn act(
        &mut self,
        world: &mut World,
        rng: &mut Rng,
        p: &RabbitParams,
        foxes: &[(usize, usize)],
    ) -> Option<Rabbit> {
        self.age += 1;
        self.cooldown = self.cooldown.saturating_sub(1);
        self.energy -= p.metabolism;

        let me = (self.x, self.y);
        let threat = if self.energy > p.desperate_energy {
            nearest_threat(foxes, me, p.vision)
        } else {
            None
        };
        if let Some(threat) = threat {
            // Fuga: ignora a fome. Dentro da toca fica quieto; senão corre para a
            // toca mais próxima ou, se não houver nenhuma perto, para longe da raposa.
            if !world.is_burrow(self.x, self.y) {
                (self.x, self.y) = match world.nearest_burrow(me, p.vision) {
                    Some(burrow) => step_toward(world, rng, me, burrow),
                    None => step_away(world, rng, me, threat),
                };
                self.energy -= p.move_cost;
            }
        } else if self.energy < p.full_energy {
            // Tem fome: come aqui se houver comida, senão procura noutra célula.
            // Os restos deixados pelo jogador vêm primeiro: comem-se sem esforço.
            if world.scraps_at(self.x, self.y) > 0.0 {
                let eaten = world.take_scraps(self.x, self.y, p.bite);
                self.energy = (self.energy + eaten * p.energy_per_biomass).min(p.energy_max);
            } else if world.biomass_at(self.x, self.y) >= p.min_food {
                let here = world.biomass_at(self.x, self.y);
                // Resposta funcional: come menos quando a comida é escassa.
                let wanted = p.bite * here / (here + p.half_saturation);
                let eaten = world.take(self.x, self.y, wanted);
                self.energy = (self.energy + eaten * p.energy_per_biomass).min(p.energy_max);
            } else {
                // A erva que vê vem antes do cheiro: só segue o nariz se nenhuma
                // vizinha tiver comida que chegue. Ao contrário, os coelhos ficavam
                // presos junto à cabana depois de os restos acabarem (o cheiro dura
                // dias), a morrer de fome com erva cheia a duas células.
                let grass = choose_move(world, rng, self.x, self.y);
                let (nx, ny) = if world.biomass_at(grass.0, grass.1) >= p.min_food {
                    grass
                } else {
                    sniff(world, rng, Scent::Food, me, p.smell_min).unwrap_or(grass)
                };
                self.x = nx;
                self.y = ny;
                self.energy -= p.move_cost;
            }
        }
        // Se não tem fome, descansa (não faz nada).

        if self.age >= p.maturity_ticks
            && self.cooldown == 0
            && self.energy >= p.repro_threshold
        {
            self.energy -= p.repro_cost;
            self.cooldown = p.repro_cooldown;
            return Some(Rabbit::newborn(self.x, self.y, p.newborn_energy));
        }
        None
    }
}

/// A raposa mais próxima que o coelho consegue ver, se houver alguma.
fn nearest_threat(
    foxes: &[(usize, usize)],
    me: (usize, usize),
    vision: usize,
) -> Option<(usize, usize)> {
    foxes
        .iter()
        .copied()
        .filter(|&f| distance(f, me) <= vision)
        .min_by_key(|&f| distance(f, me))
}

/// Escolhe a célula vizinha com mais vegetação. Um pouco de ruído aleatório
/// faz com que, onde não há nada, o coelho deambule em vez de ficar parado.
fn choose_move(world: &World, rng: &mut Rng, x: usize, y: usize) -> (usize, usize) {
    let mut best = (x, y);
    let mut best_score = f32::MIN;
    for (nx, ny) in world.neighbors(x, y).into_iter().flatten() {
        let score = world.biomass_at(nx, ny) + rng.range_f32(0.0, 1.0);
        if score > best_score {
            best_score = score;
            best = (nx, ny);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::VegetationParams;

    #[test]
    fn coelho_foge_da_raposa() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        let raposa = (5, 7);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[raposa]);
        assert_eq!(distance((coelho.x, coelho.y), raposa), 3);
    }

    #[test]
    fn coelho_com_fome_come_restos() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_scraps(5, 5, 1.0);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[]);
        assert!(world.scraps_at(5, 5) < 1.0);
        assert_eq!((coelho.x, coelho.y), (5, 5));
    }

    #[test]
    fn coelho_foge_para_a_toca() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(3, 5);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[(7, 5)]);
        // Corre em direção à toca, mesmo que isso não o afaste mais da raposa.
        assert_eq!((coelho.x, coelho.y), (4, 5));
    }

    #[test]
    fn cria_foge_desde_que_nasce() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let p = RabbitParams::default();
        let mut cria = Rabbit::newborn(5, 5, p.newborn_energy);
        let raposa = (5, 7);
        cria.act(&mut world, &mut rng, &p, &[raposa]);
        assert_eq!(distance((cria.x, cria.y), raposa), 3);
    }

    #[test]
    fn coelho_esfomeado_arrisca_e_come() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let mut coelho = Rabbit::newborn(5, 5, 20.0);
        let antes = world.biomass_at(5, 5);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[(5, 7)]);
        assert_eq!((coelho.x, coelho.y), (5, 5));
        assert!(world.biomass_at(5, 5) < antes);
    }

    #[test]
    fn coelho_nao_ve_raposa_longe() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[(11, 11)]);
        // Com fome e com comida na célula, come em vez de fugir: fica no sítio.
        assert_eq!((coelho.x, coelho.y), (5, 5));
    }
}
