//! A raposa: o primeiro predador da simulação (GDD 8.1).
//!
//! É necrófaga: se sente um cheiro forte de restos de comida, vai atrás dele
//! em vez de caçar, porque comer restos não custa nada. Senão, se vê um coelho,
//! persegue-o; se não vê nenhum, usa o nariz (restos, depois coelhos, GDD 7.6);
//! se não sente nada, deambula.
//! Evita o jogador: se ele estiver perto, afasta-se, mesmo com fome (GDD 3.1:
//! o jogador pode afugentar um predador). Também evita o cheiro que ele deixa:
//! não fica numa célula com cheiro humano forte, e quando anda pelo nariz ou
//! ao acaso não entra nelas. Quando vê um coelho, persegue-o na mesma.
//! Os mecanismos de equilíbrio da GDD 7.8 estão aqui:
//! - depois de comer fica saciada e não caça durante algum tempo;
//! - os coelhos dentro de uma toca estão a salvo, e a vegetação densa esconde os
//!   que estão fora e torna a caça mais difícil (refúgios);
//! - só se reproduz se não houver outra raposa perto (um travão de densidade),
//!   a não ser que haja restos de comida que cheguem para mais raposas.

use serde::{Deserialize, Serialize};
use crate::rabbit::Rabbit;
use crate::rng::Rng;
use crate::scent::{sniff, Scent};
use crate::world::{distance, step_away, step_toward, World};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FoxParams {
    pub energy_max: f32,
    pub energy_start: f32,
    /// Com restos aos pés, come até chegar aqui, mesmo sem fome. Antes só comia
    /// abaixo de `hunt_below`: cada dentada dava 4 de energia, a raposa ficava
    /// entre 60 e 64 para sempre e nunca chegava a `repro_threshold`, por isso
    /// as raposas acampadas na cabana não tinham crias.
    pub full_energy: f32,
    /// Acima deste valor a raposa não caça, mesmo que veja um coelho.
    /// Foi o parâmetro que mais pesou no equilíbrio. Na varredura
    /// (`eotw_cli --sweep`, 20 a 40 seeds, até 1000 dias) era estável entre 50
    /// e 70 e colapsava a partir de 75, e ficou em 60. Com o gasto e o êxito
    /// reais da raposa, 50 já precisava de migração e 65 também, quando
    /// começa com uma raposa só; 70 aguenta 60 e 80 (ver DIARIO.md e o teste
    /// `margens_de_equilibrio`).
    pub hunt_below: f32,
    /// De dia a raposa descansa e só caça com mais fome: abaixo deste valor
    /// em vez de `hunt_below`. Assim caça sobretudo de noite e ao
    /// anoitecer, como uma raposa a sério (GDD 9.2: predadores noturnos).
    /// Com o gasto real, 50 dava 47% das caçadas de dia; com 20, 4%.
    pub day_hunt_below: f32,
    /// Gasto de energia por tick quando está ativa (a caçar, a fugir, a
    /// seguir o nariz). Com cerca de 10 horas ativas e 14 a descansar
    /// (`rest_metabolism`), gasta perto de 30 por dia, e um coelho
    /// (`energy_per_rabbit`) chega para um dia e meio, como na natureza: um
    /// coelho de 0,8 a 1 kg dá a uma raposa 1 a 1,5 dias de comida (relatório
    /// "Mecânicas quantitativas dos animais"). Antes gastava 0,35 sempre, e
    /// um coelho chegava para 5 dias.
    pub metabolism: f32,
    /// O gasto por tick quando descansa (saciada ou sem fome) ou come restos
    /// parada, em vez de `metabolism`. Com o gasto de uma raposa ativa, comer
    /// restos custava mais do que eles davam e a raposa ficava presa na
    /// cabana a perder energia. Com o mesmo gasto parada e ativa, as raposas passavam o
    /// dia com fome e 47% das caçadas eram de dia.
    pub rest_metabolism: f32,
    /// Gasto extra de energia por cada célula que anda (ou por cada ataque falhado).
    pub move_cost: f32,
    /// Até quantas células de distância vê um coelho.
    pub vision: usize,
    /// Até onde vê às escuras. A raposa vê melhor no escuro do que o coelho.
    pub night_vision: usize,
    /// A esta distância do jogador, ou menos, a raposa afasta-se dele.
    pub human_fear: usize,
    /// Cheiro humano a partir do qual a raposa sai de uma célula e não entra
    /// nela quando segue o nariz ou deambula. Um jogador a andar deixa um rasto
    /// de 0,7 a 1,4; com 1 o rasto quase não contava. Com 0,5, depois de dois
    /// dias de patrulha, as raposas evitam a zona durante uns dois dias (6 horas
    /// de raposa lá dentro nas 12 horas seguintes, contra 48 sem olfato). Com
    /// 0,25 o rasto virava uma parede que quase não desaparecia.
    pub human_smell_avoid: f32,
    /// O cheiro mais fraco que a raposa sente, de comida e de coelho
    /// (GDD 7.6: olfato alto).
    pub food_smell_min: f32,
    pub prey_smell_min: f32,
    /// Acima deste cheiro de restos, a raposa larga a caça e vai aos restos,
    /// mesmo que veja um coelho. Sem isto, quase sempre via um coelho antes de
    /// cheirar a comida e só encontrava a cabana em 1 seed em 4.
    /// Abaixo de 8 há demasiada necrofagia: todas as raposas acampam na cabana,
    /// têm rivais perto e o travão de densidade deixa de as deixar ter crias
    /// (com 2 e muita comida, ficavam 2 raposas e era precisa migração).
    /// Com 8 só larga a caça quando os restos estão perto ou há muitos.
    pub scavenge_smell_min: f32,
    /// Quantas células anda por tick quando está a perseguir uma presa.
    pub chase_speed: usize,
    /// Probabilidade de apanhar um coelho ao lado, numa célula sem vegetação.
    /// Medido: a raposa acerta cerca de 23% das tentativas sobre mamíferos.
    /// Com 0,5 acertava o dobro.
    pub catch_chance: f32,
    /// Quanto a vegetação cheia reduz a probabilidade de caça (0 = nada, 1 = tudo).
    pub refuge: f32,
    /// O abrigo de uma árvore, na mesma escala da vegetação (1 = como a erva
    /// mais alta). Um coelho abrigado tem o maior dos dois.
    pub tree_refuge: f32,
    pub energy_per_rabbit: f32,
    /// Probabilidade, por hora com fome, de apanhar um roedor na erva onde
    /// está (o salto da raposa, GDD 8.1). Na erva alta acerta menos: a
    /// chance é multiplicada por `1 - mouse_cover * vegetação` (medido:
    /// cerca de 60% dos saltos acertam, um terço disso na erva alta). Os
    /// roedores não são animais da simulação: dão à raposa a dieta mista que
    /// ela tem na natureza. Sem eles, com o êxito e o metabolismo reais, as
    /// raposas ficavam em 2 por região. Com estes valores, cada raposa come
    /// cerca de 0,3 coelhos por dia e o resto do que gasta vem dos roedores.
    pub mouse_chance: f32,
    pub mouse_cover: f32,
    pub energy_per_mouse: f32,
    /// Quantos restos de comida come por tick, e quanta energia dá cada unidade.
    pub scrap_bite: f32,
    /// Carne que come de uma carcaça do lobo por hora, e a energia por kg.
    /// Um coelho de ≈1,5 kg dá `energy_per_rabbit` (45), por isso ≈30 por kg.
    pub carrion_bite: f32,
    pub energy_per_carrion: f32,
    /// Este valor decide o que os restos são para a raposa. Se forem nutritivos,
    /// a raposa enche-se na cabana, deixa de caçar e há *menos* caçadas junto à
    /// cabana do que sem comida (alimentação de diversão): com 10, 14 caçadas
    /// contra 26; com 2, depois de a raposa passar a comer até estar cheia, 3.
    /// Com 0,5, 10 restos por dia valem 5 de energia, um petisco: atraem a
    /// raposa, que depois caça os coelhos que vieram pela comida (48 caçadas).
    /// É esta a cadeia que o teste da cabana procura (GDD 18.6; ver DIARIO.md).
    pub energy_per_scrap: f32,
    /// Ticks em que fica saciada depois de comer (GDD 7.8, limites ao predador).
    pub satiety_ticks: u64,
    /// Raio do travão de densidade. Uma raposa só tem crias se não houver outra
    /// raposa a esta distância ou menos. Não é um território que ela ocupe ou
    /// defenda: as raposas andam por onde querem e caçam umas ao lado das outras;
    /// só a reprodução é travada quando há rivais perto (GDD 7.8, mecanismo 4).
    /// Sem isto, as raposas multiplicavam-se até comerem todos os coelhos e
    /// depois morriam de fome.
    pub territory_radius: usize,
    /// Energia em restos de comida, dentro do raio do travão, que dá lugar a
    /// mais uma raposa. Sem isto, com muita comida junto à cabana, as raposas
    /// juntavam-se todas lá, tinham sempre rivais perto e deixavam de ter crias,
    /// por mais comida que houvesse (o travão contava vizinhas e não comida).
    pub scrap_energy_per_extra_fox: f32,
    /// A reprodução é um evento do calendário, uma vez por ano (relatório
    /// "Raposa a fundo", B1–B3). A energia só serve de travão (design). Com
    /// 50, metade do máximo, quase não pariam: de dia a raposa só caça abaixo
    /// de 20, por isso a energia anda entre 20 e 70 e passava a época das
    /// crias quase toda abaixo de 50.
    pub repro_threshold: f32,
    /// Abaixo desta energia a raposa está esfomeada: caça o coelho que vê em
    /// vez de seguir o cheiro dos restos (design).
    pub starving_energy: f32,
    /// Energia que o parto custa de uma vez. É 0: o custo está todo na
    /// lactação (`lactation_cost`). Com 35 de uma vez e o travão a 30, uma
    /// raposa com 30 de energia paria e morria logo.
    pub repro_cost: f32,
    pub newborn_energy: f32,
    /// Uma ninhada por ano. Fica abaixo de um ano (864 ticks) de propósito:
    /// com um ano exato, a raposa que pariu a meio da época chegava à época
    /// seguinte ainda à espera, e metade das ninhadas falhava. Como a época
    /// das crias só dura 2 meses (144 ticks), 720 continua a dar uma por ano.
    pub repro_cooldown: u64,
    /// Até esta idade é juvenil (um ano, 864 ticks), para a mortalidade.
    pub juvenile_ticks: u64,
    /// Probabilidade de parir em cada tick. A simulação põe-no a partir do
    /// calendário (`CalendarParams::fox_breed_chance`); fora da época é 0.
    pub breed_per_tick: f32,
    /// Crias por ninhada e por raposa: 1, e mais uma com esta probabilidade.
    /// A ninhada é de 3,1–3,3 em Doñana e 3,6 no Ebro (Devenish-Nelson 2012,
    /// verificado); sem sexos no jogo, cada raposa conta metade: 1,8 com a
    /// do Ebro. Com a de Doñana (1,55) a população ficava à beira de λ = 1 e
    /// descia devagar; com a do Ebro aguenta-se em ≈6–7 por região.
    pub litter_extra: f32,

    /// Fração dos adultos que num ano não criam: 13,2% em Doñana
    /// (Devenish-Nelson 2012, verificado).
    pub nonbreeding: f32,
    /// O travão de densidade: cada raposa rival a `territory_radius` ou
    /// menos, além das que os restos de comida permitem, junta isto à
    /// probabilidade de não criar nesse ano. A fração de fêmeas que não criam
    /// sobe com a densidade, de ≈13% até 25–50% e mais (Devenish-Nelson
    /// 2012, verificado; o mecanismo principal da dependência da densidade).
    /// Antes era um travão total (nenhuma rival e cria, uma rival e não),
    /// que funcionava com crias a qualquer altura do ano; com uma só época
    /// por ano, duas raposas na mesma região quase nunca criavam. Medido:
    /// com 0,2 ficavam 3 raposas por região, com 0,03 ficam ≈8 e os coelhos
    /// comidos por ano são ≈33% do stock (alvo do relatório: 30–60%).
    pub rival_suppression: f32,
    /// A lactação: durante tantos ticks a mãe gasta `lactation_cost` a mais
    /// por tick, e as crias ficam na toca sem caçar nem gastar. Cerca de 5
    /// semanas, ≈84 ticks no relógio comprimido, a comer o dobro: o consumo
    /// médio de um dia (≈29,6) a dividir por 24 ≈ 1,24 a mais por tick. Como
    /// a ninhada, conta metade: sem sexos, cada raposa é meio casal, e o
    /// macho também leva comida à toca. Fica 0,62 por tick, ≈52 por ninhada
    /// e por raposa (≈104 por casal). Com 1,24 inteiro, muitas mães morriam
    /// de fome logo depois do parto.
    pub lactation_ticks: u64,
    pub lactation_cost: f32,
    /// A probabilidade de uma raposa jovem (até `juvenile_ticks`, já
    /// desmamada) partir neste tick. A simulação põe-no a partir do
    /// calendário (`CalendarParams::fox_dispersal_chance`): a tabela de
    /// Bristol, de outubro a março, ≈33% dos jovens no ano.
    pub dispersal_per_tick: f32,
    /// Uma raposa sem lugar (com mais rivais perto do que o travão aceita) e
    /// com fome (abaixo de `floater_energy`) parte com esta probabilidade
    /// por tick: as flutuantes, sem território, não ficam a morrer de fome
    /// onde as residentes caçam, vão-se embora (design). Sem isto, 30 raposas
    /// numa região comiam os coelhos todos e morriam a seguir. Com o limiar
    /// em 20 (`starving_energy`) morriam de fome antes de chegar à margem.
    /// Com 0,1 por tick e só adultas a partir: com 30, as 30 raposas
    /// iniciais deixavam 8 coelhos; com 40, 30 coelhos, e o equilíbrio fica
    /// em ≈8 raposas; com 50 ficavam ≈6. Uma mãe a amamentar não parte: antes, a lactação punha-as
    /// abaixo do limiar e em abril–maio partiam 5% das raposas por dia,
    /// deixando as crias. As jovens também não: ficam onde nasceram até à
    /// dispersão do outono (`dispersal_per_tick`); antes, as crias acabadas
    /// de desmamar, com fome e rivais à volta, partiam logo em maio.
    pub floater_leave: f32,
    pub floater_energy: f32,
    /// Células por tick de uma raposa a partir. A dispersão vai a 5–30 km,
    /// muito mais do que a paisagem: quem parte sai do mapa.
    pub dispersal_speed: usize,
    /// Raposas que chegam de fora por ano, por cada 24x24 células, a uma
    /// região vazia. Quem parte vai para fora do mapa, e de fora chegam
    /// flutuantes à procura de lugar: a paisagem é um pedaço de uma
    /// população maior. Chegam o ano todo e tanto mais quanto mais lugar há:
    /// `immigrants_per_year × (1 − raposas / full_density)`; numa região
    /// cheia não chega nenhuma. Um território vazio volta a ser ocupado
    /// depressa (o "efeito de vácuo" dos abates de raposas). Os dois valores
    /// são de design.
    pub immigrants_per_year: f32,
    /// Raposas por cada 24x24 células a partir das quais já não chega nenhuma.
    pub full_density: f32,
    /// Mortalidade de fundo (estrada, caça, doença, predadores), por tick do
    /// relógio comprimido: só a parte que não depende da densidade. A
    /// sobrevivência anual medida, 0,55 nos adultos (Suécia sem sarna,
    /// Willebrand et al. 2022, verificado) e 0,36–0,48 nos juvenis, inclui
    /// também as mortes da densidade (fome, lutas, dispersão), que no jogo
    /// vêm da fome e do travão de território. Com 0,55 e 0,42 de fundo, a
    /// população ficava presa em λ ≈ 1 e não recuperava de uma queda. Por
    /// isso: adultos 0,70 por ano (0,70^(1/864) por tick), juvenis 0,60, e
    /// nas primeiras 4 semanas (≈67 ticks) mais 20% de perda (Harris 1977,
    /// via Devenish-Nelson). Em equilíbrio, com a fome, a sobrevivência total
    /// tem de voltar a ficar perto da medida (ver os testes).
    pub adult_survival: f32,
    pub juvenile_survival: f32,
    pub cub_survival: f32,
    pub cub_ticks: u64,
    pub maturity_ticks: u64,
    /// Oito anos no relógio comprimido; com a mortalidade de fundo, quase
    /// nenhuma lá chega (esperança de vida real de 1–3 anos).
    pub max_age_ticks: u64,
}

impl Default for FoxParams {
    fn default() -> Self {
        Self {
            energy_max: 100.0,
            energy_start: 70.0,
            full_energy: 95.0,
            hunt_below: 70.0,
            day_hunt_below: 20.0,
            metabolism: 2.4,
            rest_metabolism: 0.4,
            move_cost: 0.15,
            vision: 5,
            night_vision: 4,
            human_fear: 4,
            human_smell_avoid: 0.5,
            food_smell_min: 0.5,
            prey_smell_min: 0.0,
            scavenge_smell_min: 8.0,
            chase_speed: 2,
            catch_chance: 0.25,
            refuge: 0.7,
            tree_refuge: 0.6,
            energy_per_rabbit: 45.0,
            mouse_chance: 0.45,
            mouse_cover: 0.67,
            energy_per_mouse: 4.0,
            scrap_bite: 2.0,
            carrion_bite: 0.3,
            energy_per_carrion: 30.0,
            energy_per_scrap: 0.5,
            satiety_ticks: 24,
            territory_radius: 9,
            scrap_energy_per_extra_fox: 40.0,
            repro_threshold: 30.0,
            starving_energy: 20.0,
            repro_cost: 0.0,
            newborn_energy: 40.0,
            repro_cooldown: 720,
            juvenile_ticks: 864,
            breed_per_tick: 0.0,
            litter_extra: 0.8,

            nonbreeding: 0.0,
            rival_suppression: 0.15,
            lactation_ticks: 84,
            lactation_cost: 0.62,
            adult_survival: 0.99959,
            juvenile_survival: 0.99941,
            cub_survival: 0.99667,
            cub_ticks: 67,
            dispersal_per_tick: 0.0,
            floater_leave: 0.1,
            floater_energy: 40.0,
            dispersal_speed: 2,
            immigrants_per_year: 12.0,
            full_density: 10.0,
            maturity_ticks: 10 * 72,
            max_age_ticks: 8 * 864,
        }
    }
}

/// O ponto da margem do mapa mais perto de `(x, y)`.
fn nearest_edge(world: &World, (x, y): (usize, usize)) -> (usize, usize) {
    let (w, h) = (world.width - 1, world.height - 1);
    [(x, (0, y)), (w - x, (w, y)), (y, (x, 0)), (h - y, (x, h))]
        .into_iter()
        .min_by_key(|&(d, _)| d)
        .map(|(_, c)| c)
        .expect("quatro margens")
}

/// Porque é que uma raposa está a partir.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaving {
    /// Uma jovem na época da dispersão.
    Dispersal,
    /// Uma flutuante sem lugar e com fome.
    Floater,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fox {
    pub x: usize,
    pub y: usize,
    pub energy: f32,
    pub age: u64,
    pub cooldown: u64,
    /// Ticks que faltam até voltar a ter vontade de caçar.
    pub satiety: u64,
    /// Ticks de lactação que faltam (gasta o dobro).
    #[serde(default)]
    pub lactating: u64,
    /// Está a partir, e porquê: anda para a margem mais perto e, lá chegada,
    /// sai do mapa.
    #[serde(default)]
    pub leaving: Option<Leaving>,
}

/// O que aconteceu numa raposa durante um tick.
#[derive(Debug, Default)]
pub struct FoxTick {
    pub caught: bool,
    pub babies: Vec<Fox>,
    /// Andou a seguir o cheiro de restos de comida (para o registo de eventos).
    pub followed_food: bool,
}

impl Fox {
    pub fn newborn(x: usize, y: usize, energy: f32) -> Self {
        Self {
            x,
            y,
            energy,
            age: 0,
            cooldown: 0,
            satiety: 0,
            lactating: 0,
            leaving: None,
        }
    }

    /// Um tick de vida, a procurar coelhos na lista inteira. A simulação usa
    /// `act_near`, que só olha para os coelhos perto; este serve os testes.
    #[allow(clippy::too_many_arguments)]
    pub fn act(
        &mut self,
        rabbits: &mut [Rabbit],
        world: &mut World,
        rng: &mut Rng,
        p: &FoxParams,
        rivals: usize,
        player: Option<(usize, usize)>,
    ) -> FoxTick {
        self.act_near(rabbits, None, world, rng, p, rivals, player)
    }

    /// Um tick de vida. Pode comer restos, caçar um coelho (marca-o como `eaten`)
    /// e ter uma cria. `nearby` são os índices dos coelhos perto da raposa,
    /// por ordem crescente (da grelha espacial); `None` procura em todos.
    /// `rivals` é quantas outras raposas estão a `territory_radius` ou menos;
    /// `player` é a posição do jogador, se estiver no mapa.
    #[allow(clippy::too_many_arguments)]
    pub fn act_near(
        &mut self,
        rabbits: &mut [Rabbit],
        nearby: Option<&[usize]>,
        world: &mut World,
        rng: &mut Rng,
        p: &FoxParams,
        rivals: usize,
        player: Option<(usize, usize)>,
    ) -> FoxTick {
        let mut result = FoxTick::default();
        self.age += 1;
        self.cooldown = self.cooldown.saturating_sub(1);
        self.satiety = self.satiety.saturating_sub(1);
        if self.age <= p.lactation_ticks {
            // Uma cria ainda a mamar fica na toca e não caça: quem a alimenta
            // é a mãe, que paga `lactation_cost`. Antes, a cria caçava sozinha
            // desde a primeira hora e morria de fome em poucos dias.
            return result;
        }
        self.energy -= p.metabolism;

        if self.leaving.is_none() {
            self.leaving = self.decides_to_leave(world, rng, p, rivals);
        }
        let me = (self.x, self.y);
        if self.leaving.is_some() {
            // A partir: não caça nem come, só anda para a margem mais perto,
            // e foge do jogador se ele estiver no caminho.
            for _ in 0..p.dispersal_speed {
                let here = (self.x, self.y);
                (self.x, self.y) = match player.filter(|&h| distance(h, here) <= p.human_fear) {
                    Some(human) => step_away(world, rng, here, human),
                    None => step_toward(world, rng, here, nearest_edge(world, here)),
                };
                self.energy -= p.move_cost;
            }
            return result;
        }

        // Saciada não caça, mas só enquanto tem reservas: abaixo de
        // `starving_energy` volta a ter fome. Uma mãe a amamentar comia um
        // coelho, ficava saciada um dia a gastar a lactação e acordava sem
        // tempo para caçar antes de morrer.
        let hungry = (self.satiety == 0 || self.energy < p.starving_energy) && self.energy < p.hunt_below;
        if let Some(human) = player.filter(|&h| distance(h, me) <= p.human_fear) {
            // O jogador está perto: afasta-se, e hoje não há caça nem restos.
            (self.x, self.y) = step_away(world, rng, me, human);
            self.energy -= p.move_cost;
        } else if smells_human(world, me, p) {
            // Cheira a gente aqui: sai para a vizinha onde cheira menos. Sem
            // isto o cheiro quase não contava, porque a raposa passa a maior
            // parte do tempo a descansar ou a perseguir coelhos que vê.
            let human = |c: &(usize, usize)| world.scent.at(Scent::Human, c.0, c.1);
            let cells = world.neighbors(me.0, me.1).into_iter().flatten();
            if let Some(next) = cells.min_by(|a, b| human(a).total_cmp(&human(b))) {
                (self.x, self.y) = next;
            }
            self.energy -= p.move_cost;
        } else if self.energy < p.full_energy && world.scraps_at(self.x, self.y) > 0.0 {
            // Restos de comida mesmo aqui: come até estar cheia.
            // Comer parada gasta como descansar.
            let eaten = world.take_scraps(self.x, self.y, p.scrap_bite);
            self.energy += p.metabolism - p.rest_metabolism;
            self.energy = (self.energy + eaten * p.energy_per_scrap).min(p.energy_max);
        } else if self.energy < p.full_energy && world.carcass_at(self.x, self.y) > 0.0 {
            // Uma carcaça deixada pelo lobo: carne a sério, não um petisco.
            let eaten = world.take_carcass(self.x, self.y, p.carrion_bite);
            self.energy += p.metabolism - p.rest_metabolism;
            self.energy = (self.energy + eaten * p.energy_per_carrion).min(p.energy_max);
        } else if let Some(next) = (hungry
            && !(self.energy < p.starving_energy && nearest_rabbit(rabbits, nearby, world, self.x, self.y, p.vision).is_some()))
            .then(|| sniff(world, rng, Scent::Food, (self.x, self.y), p.scavenge_smell_min))
            .flatten()
            .filter(|&c| !smells_human(world, c, p))
        {
            // Cheiro forte de restos: vai lá em vez de caçar (é necrófaga),
            // a não ser que esteja esfomeada e veja um coelho. Sem essa
            // exceção, com a cabana as raposas ficavam presas ao cheiro dos
            // restos já comidos e morriam de fome com coelhos à vista (6 de
            // 40 seeds sem raposas com 50 restos por dia).
            (self.x, self.y) = next;
            self.energy -= p.move_cost;
            result.followed_food = true;
        } else if hungry && self.mouse(world, rng, p) {
            // Apanhou um roedor na erva onde está: come-o ali.
        } else if hungry {
            match nearest_rabbit(rabbits, nearby, world, self.x, self.y, p.vision) {
                Some(i) if distance((self.x, self.y), (rabbits[i].x, rabbits[i].y)) <= 1 => {
                    result.caught = self.pounce(&mut rabbits[i], world, rng, p);
                }
                Some(i) => {
                    // Persegue: anda até `chase_speed` células e, se chegar ao lado da
                    // presa, salta logo. Se esperasse pelo tick seguinte, o coelho
                    // afastava-se sempre uma célula e a raposa nunca o apanhava.
                    let target = (rabbits[i].x, rabbits[i].y);
                    for _ in 0..p.chase_speed {
                        if distance((self.x, self.y), target) <= 1 {
                            break;
                        }
                        (self.x, self.y) = step_toward(world, rng, (self.x, self.y), target);
                        self.energy -= p.move_cost;
                    }
                    if distance((self.x, self.y), target) <= 1 {
                        result.caught = self.pounce(&mut rabbits[i], world, rng, p);
                    }
                }
                None => {
                    // Não vê nada: segue o nariz ou, se não sente nada, deambula.
                    let me = (self.x, self.y);
                    let calm = |c: &(usize, usize)| !smells_human(world, *c, p);
                    let food = sniff(world, rng, Scent::Food, me, p.food_smell_min).filter(calm);
                    result.followed_food = food.is_some();
                    (self.x, self.y) = food
                        .or_else(|| sniff(world, rng, Scent::Prey, me, p.prey_smell_min).filter(calm))
                        .unwrap_or_else(|| {
                            let all: Vec<_> =
                                world.neighbors(me.0, me.1).into_iter().flatten().collect();
                            // Ao acaso, mas longe do cheiro humano, se houver por onde.
                            let calm_cells: Vec<_> = all.iter().copied().filter(calm).collect();
                            let options = if calm_cells.is_empty() { all } else { calm_cells };
                            options[rng.range_usize(options.len())]
                        });
                    self.energy -= p.move_cost;
                }
            }
        } else {
            // Saciada ou sem fome: descansa, e a descansar gasta menos.
            self.energy += p.metabolism - p.rest_metabolism;
        }

        if self.lactating > 0 {
            self.lactating -= 1;
            self.energy -= p.lactation_cost;
        }
        if p.breed_per_tick > 0.0
            && self.age >= p.maturity_ticks
            && self.cooldown == 0
            && self.energy >= p.repro_threshold
            && rng.next_f32() < p.breed_per_tick
        {
            // Este ano já não volta a tentar, crie ou não.
            self.cooldown = p.repro_cooldown;
            let crowded = rivals.saturating_sub(self.rivals_allowed(world, p)) as f32;
            let skip = (p.nonbreeding + p.rival_suppression * crowded).min(1.0);
            if rng.next_f32() >= skip {
                self.energy -= p.repro_cost;
                self.lactating = p.lactation_ticks;
                let n = 1 + (rng.next_f32() < p.litter_extra) as usize;
                result.babies = (0..n).map(|_| Fox::newborn(self.x, self.y, p.newborn_energy)).collect();
            }
        }
        result
    }

    /// Se parte neste tick: jovem na época da dispersão, ou sem lugar e
    /// esfomeada (`dispersal_per_tick`, `floater_leave`).
    fn decides_to_leave(&self, world: &World, rng: &mut Rng, p: &FoxParams, rivals: usize) -> Option<Leaving> {
        let young = self.age < p.juvenile_ticks;
        if young && p.dispersal_per_tick > 0.0 && rng.next_f32() < p.dispersal_per_tick {
            return Some(Leaving::Dispersal);
        }
        // Uma mãe com crias na toca é residente: não parte, mesmo com fome.
        // As jovens ficam onde nasceram até à dispersão (acima); só as
        // adultas partem como flutuantes.
        (p.floater_leave > 0.0
            && self.lactating == 0
            && self.age >= p.maturity_ticks
            && self.energy < p.floater_energy
            && rivals > self.rivals_allowed(world, p)
            && rng.next_f32() < p.floater_leave)
            .then_some(Leaving::Floater)
    }

    /// Está numa margem do mapa (onde uma raposa a partir sai).
    pub fn on_edge(&self, world: &World) -> bool {
        self.x == 0 || self.y == 0 || self.x + 1 == world.width || self.y + 1 == world.height
    }

    /// Quantas rivais o travão de densidade tolera aqui: nenhuma, mais uma por
    /// cada `scrap_energy_per_extra_fox` de energia em restos dentro do raio.
    fn rivals_allowed(&self, world: &World, p: &FoxParams) -> usize {
        let scraps = world.scraps_within((self.x, self.y), p.territory_radius);
        (scraps * p.energy_per_scrap / p.scrap_energy_per_extra_fox) as usize
    }

    /// Tenta apanhar um roedor na erva onde está. Se acertar, come-o ali.
    fn mouse(&mut self, world: &World, rng: &mut Rng, p: &FoxParams) -> bool {
        let grass = world.biomass_at(self.x, self.y) / world.capacity();
        let chance = p.mouse_chance * (1.0 - p.mouse_cover * grass);
        if rng.next_f32() < chance {
            self.energy = (self.energy + p.energy_per_mouse).min(p.energy_max);
            true
        } else {
            false
        }
    }

    /// Tenta apanhar um coelho ao lado. Quanto mais vegetação houver na célula
    /// do coelho, mais difícil é (refúgio, GDD 7.8). Junto a uma árvore está
    /// tão abrigado como na erva mais alta.
    fn pounce(&mut self, rabbit: &mut Rabbit, world: &World, rng: &mut Rng, p: &FoxParams) -> bool {
        let grass = world.biomass_at(rabbit.x, rabbit.y) / world.capacity();
        let cover = if world.tree_cover(rabbit.x, rabbit.y) { grass.max(p.tree_refuge) } else { grass };
        let chance = p.catch_chance * (1.0 - p.refuge * cover);
        if rng.next_f32() < chance {
            rabbit.eaten = true;
            self.x = rabbit.x;
            self.y = rabbit.y;
            self.energy = (self.energy + p.energy_per_rabbit).min(p.energy_max);
            self.satiety = p.satiety_ticks;
            true
        } else {
            // O coelho escapou; o salto gastou energia.
            self.energy -= p.move_cost;
            false
        }
    }
}

/// A célula tem cheiro humano demais para a raposa entrar.
fn smells_human(world: &World, cell: (usize, usize), p: &FoxParams) -> bool {
    world.scent.at(Scent::Human, cell.0, cell.1) >= p.human_smell_avoid
}

/// O índice do coelho vivo mais próximo dentro do alcance da vista.
/// Os coelhos dentro de uma toca não se veem nem se podem caçar.
/// Procura só em `nearby`, se vier, ou em todos.
fn nearest_rabbit(
    rabbits: &[Rabbit],
    nearby: Option<&[usize]>,
    world: &World,
    x: usize,
    y: usize,
    vision: usize,
) -> Option<usize> {
    match nearby {
        Some(list) => nearest_of(rabbits, list.iter().copied(), world, (x, y), vision),
        None => nearest_of(rabbits, 0..rabbits.len(), world, (x, y), vision),
    }
}

fn nearest_of(
    rabbits: &[Rabbit],
    indices: impl Iterator<Item = usize>,
    world: &World,
    (x, y): (usize, usize),
    vision: usize,
) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None; // (índice, distância)
    for i in indices {
        let r = &rabbits[i];
        if r.eaten || world.is_burrow(r.x, r.y) {
            continue;
        }
        let d = distance((x, y), (r.x, r.y));
        if d <= vision && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::VegetationParams;

    fn mundo() -> World {
        let mut rng = Rng::new(1);
        World::new(12, 12, VegetationParams::default(), &mut rng)
    }

    /// Uma raposa adulta: as crias acabadas de nascer ficam na toca a mamar.
    fn adulta(x: usize, y: usize, energy: f32) -> Fox {
        Fox { age: FoxParams::default().maturity_ticks, ..Fox::newborn(x, y, energy) }
    }

    /// Parâmetros sem sorte: apanha sempre e a vegetação não protege.
    fn certeira() -> FoxParams {
        FoxParams {
            catch_chance: 1.0,
            refuge: 0.0,
            mouse_chance: 0.0,
            ..FoxParams::default()
        }
    }

    /// Sem roedores, para os testes do que a raposa faz quando não caça.
    fn sem_roedores() -> FoxParams {
        FoxParams {
            mouse_chance: 0.0,
            ..FoxParams::default()
        }
    }

    #[test]
    fn com_fome_apanha_roedores_mais_na_erva_baixa() {
        let p = FoxParams { mouse_chance: 0.6, ..FoxParams::default() };
        let contar = |erva: f32| {
            let mut apanhou = 0;
            for seed in 0..400 {
                let mut world = mundo();
                world.set_biomass(5, 5, erva);
                let mut rng = Rng::new(seed);
                let mut fox = adulta(5, 5, 40.0);
                fox.act(&mut [], &mut world, &mut rng, &p, 0, None);
                if fox.energy > 40.0 - p.metabolism {
                    apanhou += 1;
                }
            }
            apanhou
        };
        let (baixa, alta) = (contar(0.0), contar(mundo().capacity()));
        // 60% e 20% de 400.
        assert!((200..280).contains(&baixa), "erva baixa: {baixa}");
        assert!((50..110).contains(&alta), "erva alta: {alta}");
    }

    #[test]
    fn raposa_apanha_coelho_ao_lado() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut fox = adulta(5, 5, 50.0);
        let mut rabbits = vec![Rabbit::newborn(5, 6, 50.0)];
        let tick = fox.act(&mut rabbits, &mut world, &mut rng, &certeira(), 0, None);
        assert!(tick.caught);
        assert!(rabbits[0].eaten);
        assert!(fox.energy > 50.0);
        assert!(fox.satiety > 0);
    }

    #[test]
    fn raposa_saciada_nao_caca() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut fox = adulta(5, 5, 50.0);
        fox.satiety = 10;
        let mut rabbits = vec![Rabbit::newborn(5, 6, 50.0)];
        let tick = fox.act(&mut rabbits, &mut world, &mut rng, &certeira(), 0, None);
        assert!(!tick.caught);
        assert!(!rabbits[0].eaten);
    }

    #[test]
    fn raposa_persegue_coelho_que_ve() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut fox = adulta(2, 2, 50.0);
        let mut rabbits = vec![Rabbit::newborn(2, 6, 50.0)];
        fox.act(&mut rabbits, &mut world, &mut rng, &certeira(), 0, None);
        // Estava a 4 células e anda 2 por tick quando persegue.
        assert_eq!(distance((fox.x, fox.y), (2, 6)), 2);
    }

    #[test]
    fn com_rival_no_territorio_nao_tem_crias() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        // Na época das crias, e sem raposas que não criem, para o teste ser certo.
        let p = FoxParams { breed_per_tick: 1.0, nonbreeding: 0.0, ..FoxParams::default() };
        let adulta = || {
            let mut f = adulta(5, 5, 95.0);
            f.age = p.maturity_ticks;
            f.satiety = 5; // saciada: não se distrai a caçar
            f
        };
        let sozinha = adulta().act(&mut [], &mut world, &mut rng, &p, 0, None);
        // Com rivais suficientes a probabilidade de criar chega a 0.
        let rivais = (1.0 / p.rival_suppression).ceil() as usize;
        let com_rival = adulta().act(&mut [], &mut world, &mut rng, &p, rivais, None);
        assert!(!sozinha.babies.is_empty());
        assert!(com_rival.babies.is_empty());
        // Com restos que cheguem para duas raposas a mais, duas rivais já não travam.
        world.add_scraps(5, 8, 2.0 * p.scrap_energy_per_extra_fox / p.energy_per_scrap);
        let com_comida = adulta().act(&mut [], &mut world, &mut rng, &p, 2, None);
        let demasiadas = adulta().act(&mut [], &mut world, &mut rng, &p, 2 + rivais, None);
        assert!(!com_comida.babies.is_empty());
        assert!(demasiadas.babies.is_empty());
    }

    #[test]
    fn raposa_com_fome_come_restos() {
        let mut world = mundo();
        world.add_scraps(5, 5, 5.0);
        let mut rng = Rng::new(1);
        let mut fox = adulta(5, 5, 40.0);
        fox.act(&mut [], &mut world, &mut rng, &FoxParams::default(), 0, None);
        assert!(fox.energy > 40.0);
        assert!(world.scraps_at(5, 5) < 5.0);
        assert_eq!((fox.x, fox.y), (5, 5)); // ficou a comer
    }

    /// Sem fome mas com restos aos pés, come até estar cheia: senão ficava
    /// presa logo acima de `hunt_below` e nunca tinha energia para ter crias.
    #[test]
    fn raposa_com_restos_come_ate_estar_cheia() {
        let mut world = mundo();
        world.add_scraps(5, 5, 1000.0);
        let mut rng = Rng::new(1);
        let p = FoxParams::default();
        let mut fox = adulta(5, 5, p.hunt_below + 1.0);
        for _ in 0..200 {
            fox.act(&mut [], &mut world, &mut rng, &p, 0, None);
        }
        assert!(fox.energy >= p.repro_threshold, "ficou em {}", fox.energy);
    }

    #[test]
    fn raposa_necrofaga_prefere_restos_a_caca() {
        let mut world = mundo();
        // Cheiro forte de restos a oeste; um coelho à vista a leste.
        world.scent.deposit(Scent::Food, 3, 5, 50.0);
        let mut rng = Rng::new(1);
        let mut fox = adulta(4, 5, 40.0);
        let mut rabbits = vec![Rabbit::newborn(8, 5, 50.0)];
        fox.act(&mut rabbits, &mut world, &mut rng, &certeira(), 0, None);
        assert_eq!((fox.x, fox.y), (3, 5));
    }

    #[test]
    fn raposa_foge_do_jogador_mesmo_com_fome() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut fox = adulta(5, 5, 30.0);
        // Um coelho ao lado, mas o jogador também está perto.
        let mut rabbits = vec![Rabbit::newborn(5, 6, 50.0)];
        let jogador = (5, 3);
        fox.act(&mut rabbits, &mut world, &mut rng, &certeira(), 0, Some(jogador));
        assert!(!rabbits[0].eaten);
        assert_eq!(distance((fox.x, fox.y), jogador), 3);
    }

    #[test]
    fn raposa_nao_entra_onde_cheira_a_gente() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        // Cheiro humano forte em três das quatro vizinhas: só resta (6, 5).
        for c in [(5, 4), (5, 6), (4, 5)] {
            world.scent.deposit(Scent::Human, c.0, c.1, 5.0);
        }
        for _ in 0..20 {
            let mut fox = adulta(5, 5, 40.0);
            fox.act(&mut [], &mut world, &mut rng, &sem_roedores(), 0, None);
            assert_eq!((fox.x, fox.y), (6, 5));
        }
    }

    #[test]
    fn raposa_sai_de_onde_cheira_a_gente() {
        let mut world = mundo();
        let mut rng = Rng::new(1);
        world.scent.deposit(Scent::Human, 5, 5, 5.0);
        world.scent.deposit(Scent::Human, 5, 4, 3.0);
        world.scent.deposit(Scent::Human, 4, 5, 3.0);
        world.scent.deposit(Scent::Human, 5, 6, 2.0);
        // Saciada: sem o cheiro ficava a descansar. Sai para onde cheira menos.
        let mut fox = adulta(5, 5, 90.0);
        fox.satiety = 10;
        fox.act(&mut [], &mut world, &mut rng, &FoxParams::default(), 0, None);
        assert_eq!((fox.x, fox.y), (6, 5));
    }

    #[test]
    fn coelho_na_toca_esta_a_salvo() {
        let mut world = mundo();
        world.add_burrow(5, 6);
        let mut rng = Rng::new(1);
        let mut fox = adulta(5, 5, 50.0);
        let mut rabbits = vec![Rabbit::newborn(5, 6, 50.0)];
        let tick = fox.act(&mut rabbits, &mut world, &mut rng, &certeira(), 0, None);
        assert!(!tick.caught);
        assert!(!rabbits[0].eaten);
    }

    #[test]
    fn a_arvore_protege_os_coelhos() {
        use crate::resources::{Resource, ResourceKind};
        let mut world = mundo();
        // Duas células sem erva, uma delas ao lado de uma árvore.
        world.take(0, 1, 1000.0);
        world.take(6, 7, 1000.0);
        world.add_resource(Resource { x: 7, y: 7, kind: ResourceKind::Tree, amount: 4.0 });
        let p = sem_roedores();
        let mut rng = Rng::new(9);
        let (mut expostos, mut abrigados) = (0, 0);
        for _ in 0..1000 {
            let mut fox = adulta(0, 0, 50.0);
            if fox.act(&mut [Rabbit::newborn(0, 1, 50.0)], &mut world, &mut rng, &p, 0, None).caught {
                expostos += 1;
            }
            let mut fox = adulta(6, 6, 50.0);
            if fox.act(&mut [Rabbit::newborn(6, 7, 50.0)], &mut world, &mut rng, &p, 0, None).caught {
                abrigados += 1;
            }
        }
        // Sem erva, a 25%; junto à árvore, a 25% × (1 − 0,7 × 0,6) = 14,5%.
        assert!(abrigados * 3 < expostos * 2, "abrigados {abrigados}, expostos {expostos}");
    }

    #[test]
    fn vegetacao_densa_protege_os_coelhos() {
        let mut world = mundo();
        world.take(0, 1, 1000.0); // célula vazia: coelho exposto
        let p = sem_roedores();
        let mut rng = Rng::new(9);
        let mut apanhados_expostos = 0;
        let mut apanhados_escondidos = 0;
        for _ in 0..1000 {
            let mut fox = adulta(0, 0, 50.0);
            let mut exposto = vec![Rabbit::newborn(0, 1, 50.0)];
            if fox.act(&mut exposto, &mut world, &mut rng, &p, 0, None).caught {
                apanhados_expostos += 1;
            }
            // Célula (6, 7) começa com bastante vegetação (entre 40% e 100%).
            let mut fox = adulta(6, 6, 50.0);
            let mut escondido = vec![Rabbit::newborn(6, 7, 50.0)];
            if fox.act(&mut escondido, &mut world, &mut rng, &p, 0, None).caught {
                apanhados_escondidos += 1;
            }
        }
        assert!(
            apanhados_escondidos < apanhados_expostos,
            "escondidos {apanhados_escondidos}, expostos {apanhados_expostos}"
        );
    }

    /// Uma cria a mamar fica na toca: não anda, não caça e não gasta (quem
    /// paga é a mãe, com `lactation_cost`).
    #[test]
    fn cria_a_mamar_fica_na_toca() {
        let p = certeira();
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut cria = Fox::newborn(5, 5, 40.0);
        let mut rabbits = vec![Rabbit::newborn(5, 6, 50.0)];
        for _ in 0..p.lactation_ticks - 1 {
            let tick = cria.act(&mut rabbits, &mut world, &mut rng, &p, 0, None);
            assert!(!tick.caught);
        }
        assert_eq!((cria.x, cria.y, cria.energy), (5, 5, 40.0));
        assert!(!rabbits[0].eaten);
    }

    /// Uma raposa a partir anda para a margem mais perto e não caça.
    #[test]
    fn a_partir_vai_para_a_margem() {
        let p = certeira();
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut fox = adulta(3, 6, 50.0);
        fox.leaving = Some(Leaving::Dispersal);
        let mut rabbits = vec![Rabbit::newborn(3, 7, 50.0)];
        fox.act(&mut rabbits, &mut world, &mut rng, &p, 0, None);
        assert!(!rabbits[0].eaten);
        fox.act(&mut rabbits, &mut world, &mut rng, &p, 0, None);
        assert_eq!(fox.x, 0, "anda {} células por tick para a margem esquerda", p.dispersal_speed);
        assert!(fox.on_edge(&world));
    }

    /// Partem como flutuantes as adultas com fome e rivais a mais; não partem
    /// as que têm comida, as sozinhas, as mães a amamentar nem as jovens.
    #[test]
    fn quem_parte_como_flutuante() {
        let p = FoxParams { floater_leave: 1.0, ..sem_roedores() };
        let parte = |mut fox: Fox, rivals: usize| {
            let mut world = mundo();
            let mut rng = Rng::new(1);
            fox.act(&mut [], &mut world, &mut rng, &p, rivals, None);
            fox.leaving
        };
        let fome = p.floater_energy - 5.0;
        assert_eq!(parte(adulta(5, 5, fome), 2), Some(Leaving::Floater));
        assert_eq!(parte(adulta(5, 5, p.floater_energy + 10.0), 2), None);
        assert_eq!(parte(adulta(5, 5, fome), 0), None);
        let mae = Fox { lactating: 10, ..adulta(5, 5, fome) };
        assert_eq!(parte(mae, 2), None);
        let jovem = Fox { age: p.lactation_ticks + 10, ..Fox::newborn(5, 5, fome) };
        assert_eq!(parte(jovem, 2), None);
    }

    /// Na época da dispersão, as jovens partem; as adultas não.
    #[test]
    fn as_jovens_dispersam() {
        let p = FoxParams { dispersal_per_tick: 1.0, ..sem_roedores() };
        let mut world = mundo();
        let mut rng = Rng::new(1);
        let mut jovem = Fox { age: p.lactation_ticks + 10, ..Fox::newborn(5, 5, 80.0) };
        let mut adulta = Fox { age: p.juvenile_ticks, ..Fox::newborn(5, 5, 80.0) };
        jovem.act(&mut [], &mut world, &mut rng, &p, 0, None);
        adulta.act(&mut [], &mut world, &mut rng, &p, 0, None);
        assert_eq!(jovem.leaving, Some(Leaving::Dispersal));
        assert_eq!(adulta.leaving, None);
    }
}
