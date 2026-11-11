//! A fome e a saúde do jogador (GDD 9.3 e 18.2: "Saúde e alimentação
//! simples").
//!
//! Versão simplificada, sem pesquisa ainda: os números são decisões de jogo,
//! escolhidas para a fome pesar sem obrigar a parar de poucos em poucos
//! minutos (GDD 9.3). A comida vem da erva: ervas, raízes e cogumelos que o
//! jogador procura (tecla F) e come depois (tecla C). Procurar gasta erva,
//! por isso o jogador compete com os coelhos.

/// Os números da fome e da saúde. Ambas vão de 0 a 100.
#[derive(Clone, Debug)]
pub struct NeedsParams {
    /// Quanto a saciedade desce por hora. Com 1,4, cheio fica com fome ao fim
    /// de três dias de jogo, e por dia gasta 34: cerca de uma refeição e meia.
    pub food_per_hour: f32,
    /// Saúde perdida por hora com a saciedade a 0. Com 2, morre-se de fome
    /// ao fim de pouco mais de dois dias sem comer nada.
    pub starving_damage: f32,
    /// Saúde recuperada por hora com a saciedade acima de `heal_above`.
    pub heal_per_hour: f32,
    pub heal_above: f32,
    /// Saciedade que uma porção de comida dá.
    pub meal: f32,
    /// Vegetação que cada porção de comida gasta. A erva só dá comida se
    /// tiver pelo menos isto.
    pub forage_biomass: f32,
    /// Horas de procura por porção.
    pub forage_hours: u64,
    /// Com a saúde a 0 o jogador desmaia e acorda com isto de saúde e de
    /// saciedade (ainda não há morte nem fim de jogo).
    pub wake_with: f32,
}

impl Default for NeedsParams {
    fn default() -> Self {
        Self {
            food_per_hour: 1.4,
            starving_damage: 2.0,
            heal_per_hour: 1.0,
            heal_above: 30.0,
            meal: 25.0,
            forage_biomass: 3.0,
            forage_hours: 1,
            wake_with: 25.0,
        }
    }
}

/// A fome e a saúde do jogador.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Needs {
    /// Saciedade: 100 é cheio, 0 é fome.
    pub food: f32,
    pub health: f32,
    /// Quantas vezes desmaiou.
    pub faints: u32,
}

impl Default for Needs {
    fn default() -> Self {
        Self { food: 80.0, health: 100.0, faints: 0 }
    }
}

impl Needs {
    /// Uma hora. Devolve `true` se o jogador desmaiou nesta hora.
    pub fn tick(&mut self, p: &NeedsParams) -> bool {
        self.food = (self.food - p.food_per_hour).max(0.0);
        if self.food == 0.0 {
            self.health -= p.starving_damage;
        } else if self.food > p.heal_above {
            self.health += p.heal_per_hour;
        }
        self.health = self.health.min(100.0);
        if self.health <= 0.0 {
            self.health = p.wake_with;
            self.food = p.wake_with;
            self.faints += 1;
            return true;
        }
        false
    }

    pub fn eat(&mut self, p: &NeedsParams) {
        self.food = (self.food + p.meal).min(100.0);
    }

    /// Uma palavra para o painel.
    pub fn hunger_word(&self) -> &'static str {
        match self.food {
            f if f >= 70.0 => "satisfeito",
            f if f >= 30.0 => "com alguma fome",
            f if f > 0.0 => "com fome",
            _ => "esfomeado",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheio_aguenta_tres_dias_e_depois_perde_saude() {
        let p = NeedsParams::default();
        let mut n = Needs { food: 100.0, ..Needs::default() };
        for _ in 0..70 {
            n.tick(&p);
        }
        assert!(n.food > 0.0 && n.health == 100.0, "{n:?}");
        for _ in 0..10 {
            n.tick(&p);
        }
        assert_eq!(n.food, 0.0);
        assert!(n.health < 100.0);
    }

    #[test]
    fn sem_comer_desmaia_e_acorda_fraco() {
        let p = NeedsParams::default();
        let mut n = Needs { food: 0.0, ..Needs::default() };
        let horas = (0..100).position(|_| n.tick(&p)).unwrap() + 1;
        assert_eq!(horas, 50, "100 de saúde a 2 por hora");
        assert_eq!((n.food, n.health, n.faints), (p.wake_with, p.wake_with, 1));
    }

    #[test]
    fn comer_tira_a_fome_e_a_saude_volta() {
        let p = NeedsParams::default();
        let mut n = Needs { food: 10.0, health: 50.0, faints: 0 };
        n.eat(&p);
        n.eat(&p);
        assert_eq!(n.food, 60.0);
        n.tick(&p);
        assert_eq!(n.health, 51.0);
    }
}
