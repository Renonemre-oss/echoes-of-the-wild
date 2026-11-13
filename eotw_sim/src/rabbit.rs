//! O coelho: o primeiro animal da simulação.
//!
//! Tem uma necessidade (energia), procura vegetação, come e reproduz-se quando está
//! bem alimentado. Se vê uma raposa, foge para a toca mais próxima ou, se não houver
//! nenhuma perto, para longe da raposa. A fuga tem prioridade sobre a fome, exceto
//! quando o coelho está a morrer de fome: aí arrisca e vai comer (GDD 17.3).
//! Com fome e sem comida onde está, vai para a célula vizinha com mais
//! vegetação; se nenhuma tiver que chegue, segue o cheiro de restos de comida.
//! Ao meio-dia, os coelhos com energia entre `midday_rest_above` e
//! `rest_cap` descansam na toca: lá não gastam energia e recuperam um pouco,
//! como os coelhos bravos, que digerem na lura de dia (cecotrofia) e comem
//! ao crepúsculo e de noite. Os outros continuam ativos.

use serde::{Deserialize, Serialize};
use crate::rng::Rng;
use crate::scent::{sniff, Scent};
use crate::world::{distance, step_away, step_toward, World};

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    /// A reprodução é um evento do calendário (relatório "Coelho a fundo",
    /// B1–B3). A energia só serve de travão: abaixo disto não pare (com 70–80%
    /// da ração, os coelhos param de criar, Boyd & Bray 1989).
    pub repro_threshold: f32,
    /// Energia que o parto e a lactação custam à mãe.
    pub repro_cost: f32,
    pub newborn_energy: f32,
    /// Ticks entre dois partos: um mês do calendário (72 ticks), a gestação
    /// de ≈30 dias no relógio comprimido do ano de 36 dias.
    pub repro_cooldown: u64,
    /// Probabilidade de parir em cada tick. Não é um número fixo: a
    /// simulação põe-no em cada hora a partir da época de cria do mês
    /// (`CalendarParams::rabbit_breed_chance` / ticks do mês). Fora disso é 0.
    pub breed_per_tick: f32,
    /// Crias por parto e por coelho: 1, e mais uma com esta probabilidade.
    /// A ninhada real é de 3,5 (Villafuerte, verificado); como o jogo não
    /// tem sexos, cada coelho conta metade: 1,75 em média.
    pub litter_extra: f32,
    /// Idade adulta: 4 meses no relógio comprimido (4 × 72 ticks), a do
    /// coelho-bravo do sul (Villafuerte, verificado).
    pub maturity_ticks: u64,
    /// Mortalidade dos jovens dependente da densidade de adultos (Calvete
    /// 2006, verificado): d = 0,041 × D/K por dia real, para os jovens das 4
    /// semanas aos 8 meses (≈67 a 576 ticks no relógio comprimido). Como esses
    /// ≈212 dias reais cabem em 509 ticks, por tick daria 0,041 × 212 / 509 ≈
    /// 0,017 × D/K. Mas o d de Calvete inclui todas as causas, também os
    /// predadores, e no jogo as raposas já matam jovens à parte: com 0,017 a
    /// predação contava duas vezes e os coelhos não chegavam para as raposas.
    /// Fica 0,006, cerca de um terço (design: a parte que não é predação).
    /// D são os adultos da região e K `adult_capacity` por cada 576 células
    /// (24×24), o botão da compressão: com 300, o equilíbrio fica em ≈260
    /// coelhos por região, como antes.
    pub juvenile_from: u64,
    pub juvenile_until: u64,
    pub juvenile_mortality: f32,
    pub adult_capacity: f32,
    /// Seis anos no relógio comprimido. O recorde em liberdade é de 9,5 anos
    /// (Peacock & Sinclair 2023, resumo); quase nenhum lá chega.
    pub max_age_ticks: u64,
    /// Até quantas células de distância vê uma raposa. A raposa vê mais longe,
    /// por isso às vezes chega perto antes de o coelho dar por ela.
    pub vision: usize,
    /// Até onde vê às escuras (GDD 7.6: a visão diminui de noite).
    pub night_vision: usize,
    /// Até onde ouve os passos do jogador, de dia e de noite. A raposa anda
    /// calada e só se vê; uma pessoa a andar ouve-se. Medido: a lebre foge de
    /// uma pessoa a cerca de 70 m (7 células). Sem isto, com os coelhos ativos
    /// de noite, quase não davam pelo jogador às escuras.
    pub player_hearing: usize,
    /// Com esta energia ou mais, o coelho vai descansar para a toca em vez de
    /// comer. Fora do meio-dia não descansa (`f32::MAX`): come até `full_energy`.
    pub rest_above: f32,
    /// O mesmo, ao meio-dia (GDD 7.1: alternar atividade e repouso). Os
    /// coelhos com menos energia continuam ativos, como no resto do dia.
    /// Antes era de noite. Experimentei pôr quase todos na toca ao meio-dia
    /// (de 30 de energia para cima, sem teto), como os coelhos bravos: os
    /// coelhos passavam a comer só à volta das tocas, rapavam a erva lá e a
    /// população caía para menos de 10 (ver DIARIO.md).
    pub midday_rest_above: f32,
    /// Até quantas células vai buscar uma toca para descansar.
    pub rest_range: usize,
    /// Energia que recupera por tick a descansar na toca. A descansar não gasta
    /// o `metabolism`: só os coelhos ativos perdem energia.
    pub rest_recovery: f32,
    /// Só descansa abaixo desta energia: ao chegar aqui sai da toca e vai
    /// comer. Para chegar a `repro_threshold` tem de comer.
    pub rest_cap: f32,
    /// Abaixo desta energia o coelho deixa de fugir e arrisca ir comer. Sem isto,
    /// com raposas por perto, morriam mais coelhos de fome (escondidos, com a
    /// vegetação cheia à volta) do que caçados. Tem de ficar abaixo de
    /// `newborn_energy`.
    pub desperate_energy: f32,
    /// Quanto vale, ao escolher para onde ir comer, uma célula abrigada por
    /// uma árvore (na mesma escala da vegetação, 0 a `capacity`). Onde se
    /// cortam árvores, os coelhos passam a ir menos e a erva cresce mais.
    pub cover_bonus: f32,
    /// Quanto vale a menos a erva numa clareira (à volta de uma árvore
    /// cortada), para decidir se come ali e para onde ir. Os coelhos sentem-se
    /// expostos e vão comer para outro sítio. Num mundo sem cortes não há
    /// clareiras, por isso isto não mexe no equilíbrio.
    pub clearing_penalty: f32,
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
            repro_threshold: 60.0,
            repro_cost: 40.0,
            // Acima de `desperate_energy`, para as crias fugirem desde que nascem.
            // Com 30 (igual ao limiar) nasciam já "desesperadas" e não fugiam.
            newborn_energy: 35.0,
            repro_cooldown: 72,
            breed_per_tick: 0.0,
            litter_extra: 0.75,
            maturity_ticks: 4 * 72,
            juvenile_from: 67,
            juvenile_until: 8 * 72,
            juvenile_mortality: 0.006,
            adult_capacity: 300.0,
            max_age_ticks: 6 * 864,
            vision: 3,
            night_vision: 2,
            player_hearing: 3,
            // O maior número, e não o infinito, que o JSON não guarda.
            rest_above: f32::MAX,
            midday_rest_above: 40.0,
            rest_range: 6,
            rest_recovery: 1.5,
            rest_cap: 60.0,
            desperate_energy: 30.0,
            smell_min: 1.0,
            cover_bonus: 1.0,
            clearing_penalty: 6.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

    /// Um tick de vida. `threats` são as posições das raposas e `player` a
    /// do jogador, se estiver no mapa: o coelho teme os dois.
    /// Devolve as crias, se o coelho pariu neste tick.
    pub fn act(
        &mut self,
        world: &mut World,
        rng: &mut Rng,
        p: &RabbitParams,
        threats: &[(usize, usize)],
        player: Option<(usize, usize)>,
    ) -> Vec<Rabbit> {
        self.age += 1;
        self.cooldown = self.cooldown.saturating_sub(1);
        self.energy -= p.metabolism;

        let me = (self.x, self.y);
        let threat = if self.energy > p.desperate_energy {
            let heard = player.filter(|&h| distance(h, me) <= p.player_hearing.max(p.vision));
            nearest_threat(threats, me, p.vision).into_iter().chain(heard).min_by_key(|&t| distance(t, me))
        } else {
            None
        };
        // Um coelho com energia que chegue para descansar, e uma toca perto.
        let rest = if self.energy >= p.rest_above && self.energy < p.rest_cap {
            world.nearest_burrow(me, p.rest_range)
        } else {
            None
        };
        if let Some(threat) = threat {
            // Fuga: ignora a fome. Dentro da toca fica quieto; senão corre para a
            // toca mais próxima ou, se não houver nenhuma perto, para longe da ameaça.
            // Corre para a toca mesmo que ela fique do lado da ameaça, como um
            // coelho a sério. Experimentei só aceitar tocas que não o aproximassem
            // da ameaça, mas as tocas deixaram de proteger e, a 1000 dias, os
            // coelhos chegavam a 4 (ver DIARIO.md).
            if !world.is_burrow(self.x, self.y) {
                (self.x, self.y) = match world.nearest_burrow(me, p.vision) {
                    Some(burrow) => step_toward(world, rng, me, burrow),
                    None => step_away(world, rng, me, threat),
                };
                self.energy -= p.move_cost;
            }
        } else if let Some(burrow) = rest {
            // Descanso: na toca não gasta energia e recupera um pouco; senão vai
            // para lá, e enquanto anda gasta como um coelho ativo.
            if world.is_burrow(self.x, self.y) {
                self.energy = (self.energy + p.metabolism + p.rest_recovery).min(p.energy_max);
            } else {
                (self.x, self.y) = step_toward(world, rng, me, burrow);
                self.energy -= p.move_cost;
            }
        } else if self.energy < p.full_energy {
            // Tem fome: come aqui se houver comida, senão procura noutra célula.
            // Os restos deixados pelo jogador vêm primeiro: comem-se sem esforço.
            if world.scraps_at(self.x, self.y) > 0.0 {
                let eaten = world.take_scraps(self.x, self.y, p.bite);
                self.energy = (self.energy + eaten * p.energy_per_biomass).min(p.energy_max);
            } else if edible(world, (self.x, self.y), p) >= p.min_food {
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
                let grass = choose_move(world, rng, self.x, self.y, p);
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
            && p.breed_per_tick > 0.0
            && rng.next_f32() < p.breed_per_tick
        {
            self.energy -= p.repro_cost;
            self.cooldown = p.repro_cooldown;
            let n = 1 + (rng.next_f32() < p.litter_extra) as usize;
            return (0..n).map(|_| Rabbit::newborn(self.x, self.y, p.newborn_energy)).collect();
        }
        Vec::new()
    }
}

/// A ameaça mais próxima que o coelho consegue ver, se houver alguma.
fn nearest_threat(
    threats: &[(usize, usize)],
    me: (usize, usize),
    vision: usize,
) -> Option<(usize, usize)> {
    threats
        .iter()
        .copied()
        .filter(|&f| distance(f, me) <= vision)
        .min_by_key(|&f| distance(f, me))
}

/// A erva que o coelho conta para decidir se come ali: numa clareira vale
/// `clearing_penalty` a menos, porque ele se sente exposto.
fn edible(world: &World, (x, y): (usize, usize), p: &RabbitParams) -> f32 {
    let grass = world.biomass_at(x, y);
    if world.clearing(x, y) {
        grass - p.clearing_penalty
    } else {
        grass
    }
}

/// O que uma célula vale para ir lá comer: `edible`, mais `cover_bonus` se
/// uma árvore a abriga.
fn appeal(world: &World, cell: (usize, usize), p: &RabbitParams) -> f32 {
    let bonus = if world.tree_cover(cell.0, cell.1) { p.cover_bonus } else { 0.0 };
    edible(world, cell, p) + bonus
}

/// Escolhe a célula vizinha que vale mais para comer (`appeal`). Um pouco
/// de ruído aleatório faz com que, onde não há nada, o coelho deambule em
/// vez de ficar parado.
fn choose_move(world: &World, rng: &mut Rng, x: usize, y: usize, p: &RabbitParams) -> (usize, usize) {
    let mut best = (x, y);
    let mut best_score = f32::MIN;
    for (nx, ny) in world.neighbors(x, y).into_iter().flatten() {
        let score = appeal(world, (nx, ny), p) + rng.range_f32(0.0, 1.0);
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
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[raposa], None);
        assert_eq!(distance((coelho.x, coelho.y), raposa), 3);
    }

    #[test]
    fn coelho_com_fome_come_restos() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_scraps(5, 5, 1.0);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[], None);
        assert!(world.scraps_at(5, 5) < 1.0);
        assert_eq!((coelho.x, coelho.y), (5, 5));
    }

    #[test]
    fn coelho_foge_para_a_toca() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(3, 5);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[(7, 5)], None);
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
        cria.act(&mut world, &mut rng, &p, &[raposa], None);
        assert_eq!(distance((cria.x, cria.y), raposa), 3);
    }

    #[test]
    fn coelho_esfomeado_arrisca_e_come() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let mut coelho = Rabbit::newborn(5, 5, 20.0);
        let antes = world.biomass_at(5, 5);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[(5, 7)], None);
        assert_eq!((coelho.x, coelho.y), (5, 5));
        assert!(world.biomass_at(5, 5) < antes);
    }

    /// Os parâmetros do meio-dia, como a simulação os põe.
    fn ao_meio_dia() -> RabbitParams {
        let p = RabbitParams::default();
        RabbitParams {
            rest_above: p.midday_rest_above,
            ..p
        }
    }

    #[test]
    fn ao_meio_dia_descansa_na_toca_e_recupera() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(5, 5);
        let mut coelho = Rabbit::newborn(5, 5, 45.0);
        let antes = world.biomass_at(5, 5);
        coelho.act(&mut world, &mut rng, &ao_meio_dia(), &[], None);
        assert_eq!((coelho.x, coelho.y), (5, 5));
        assert_eq!(coelho.energy, 45.0 + ao_meio_dia().rest_recovery);
        // A descansar não come.
        assert_eq!(world.biomass_at(5, 5), antes);
    }

    #[test]
    fn ao_meio_dia_vai_para_a_toca_descansar() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(3, 5);
        let mut coelho = Rabbit::newborn(5, 5, 45.0);
        coelho.act(&mut world, &mut rng, &ao_meio_dia(), &[], None);
        assert_eq!((coelho.x, coelho.y), (4, 5));
    }

    #[test]
    fn ao_meio_dia_com_pouca_energia_continua_a_comer() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(5, 5);
        let p = ao_meio_dia();
        let mut coelho = Rabbit::newborn(5, 5, p.midday_rest_above - 5.0);
        let antes = world.biomass_at(5, 5);
        coelho.act(&mut world, &mut rng, &p, &[], None);
        assert!(world.biomass_at(5, 5) < antes);
    }

    /// Acima de `rest_cap` sai da toca e vai comer: só a comer chega a
    /// `repro_threshold`.
    #[test]
    fn ao_meio_dia_acima_do_teto_vai_comer() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(5, 5);
        let p = ao_meio_dia();
        let mut coelho = Rabbit::newborn(5, 5, p.rest_cap + 5.0);
        let antes = world.biomass_at(5, 5);
        coelho.act(&mut world, &mut rng, &p, &[], None);
        assert!(world.biomass_at(5, 5) < antes);
    }

    #[test]
    fn fora_do_meio_dia_nao_descansa() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        world.add_burrow(5, 5);
        let mut coelho = Rabbit::newborn(5, 5, 45.0);
        let antes = world.biomass_at(5, 5);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[], None);
        assert!(world.biomass_at(5, 5) < antes);
    }

    /// Com a mesma erva à volta, o coelho com fome vai para a célula
    /// abrigada por uma árvore.
    #[test]
    fn com_a_mesma_erva_prefere_o_abrigo() {
        use crate::resources::{Resource, ResourceKind};
        let mut abrigo = 0;
        for seed in 0..200 {
            let mut rng = Rng::new(seed);
            let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
            for y in 0..12 {
                for x in 0..12 {
                    world.set_biomass(x, y, if (x, y) == (5, 5) { 0.0 } else { 5.0 });
                }
            }
            // A árvore abriga (6, 5), a vizinha da direita, e não as outras.
            world.add_resource(Resource { x: 7, y: 5, kind: ResourceKind::Tree, amount: 4.0 });
            let mut coelho = Rabbit::newborn(5, 5, 50.0);
            coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[], None);
            if (coelho.x, coelho.y) == (6, 5) {
                abrigo += 1;
            }
        }
        // Ao acaso seria 1 em 4 (50); com o abrigo, muito mais.
        assert!(abrigo > 100, "foi para o abrigo {abrigo} vezes em 200");
    }

    /// De noite vê a 2 células, mas ouve o jogador a 3; a raposa não se ouve.
    #[test]
    fn de_noite_ouve_o_jogador_mas_nao_a_raposa() {
        let p = RabbitParams { vision: RabbitParams::default().night_vision, ..RabbitParams::default() };
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &p, &[], Some((5, 8)));
        assert_eq!(distance((coelho.x, coelho.y), (5, 8)), 4, "fugiu do jogador");
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &p, &[(5, 8)], None);
        assert_eq!((coelho.x, coelho.y), (5, 5), "não deu pela raposa e comeu");
    }

    #[test]
    fn coelho_nao_ve_raposa_longe() {
        let mut rng = Rng::new(1);
        let mut world = World::new(12, 12, VegetationParams::default(), &mut rng);
        let mut coelho = Rabbit::newborn(5, 5, 50.0);
        coelho.act(&mut world, &mut rng, &RabbitParams::default(), &[(11, 11)], None);
        // Com fome e com comida na célula, come em vez de fugir: fica no sítio.
        assert_eq!((coelho.x, coelho.y), (5, 5));
    }
}
