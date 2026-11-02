//! O jogo: a simulação do `eotw_sim` desenhada numa janela com Bevy (etapa 4).
//!
//! A simulação não sabe que o Bevy existe. Este programa só faz três coisas:
//! avança a simulação ao ritmo escolhido, desenha o que ela calculou e lê o
//! teclado. Toda a ecologia continua em `eotw_sim`.
//!
//! Exemplos:
//!   cargo run -p eotw_game
//!   cargo run -p eotw_game -- --seed 7 --cabin
//!   cargo run -p eotw_game -- --screenshot janela.png
//!   cargo run -p eotw_game -- --smell   (começa com a vista do cheiro ligada)

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use eotw_sim::{CabinParams, Config, Scent, Simulation, TICKS_PER_DAY};

/// Tamanho de uma célula do mapa no ecrã, em píxeis.
const CELL: f32 = 26.0;

/// Largura reservada ao painel de texto, à esquerda do mapa.
const PANEL_WIDTH: f32 = 330.0;

/// Tempo entre passos do jogador com a tecla carregada, em segundos.
const PLAYER_STEP_SECS: f32 = 0.12;

/// Ritmos possíveis, em horas de jogo por segundo real.
const SPEEDS: [f32; 6] = [1.0, 4.0, 12.0, 24.0, 72.0, 240.0];

fn main() {
    let options = Options::from_args();
    let smell = options.smell;
    let mut config = Config {
        seed: options.seed,
        ..Config::default()
    };
    if options.cabin {
        config.cabin = Some(CabinParams::centered(config.width, config.height, 10.0));
    }
    // O jogador chega depois do aquecimento, a um mundo já em equilíbrio,
    // ao lado da cabana (ou ao centro, se não houver cabana).
    let mut sim = Simulation::warmed_up(config);
    let (w, h) = (sim.world.width, sim.world.height);
    let start = sim.config().cabin.as_ref().map_or((w / 2, h / 2), |c| (c.x, c.y + 1));
    sim.place_player(Some(start));

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Echoes of the Wild".into(),
                resolution: (960.0_f32, 720.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.10, 0.11, 0.10)))
        .insert_resource(World(sim))
        .insert_resource(Clock {
            speed: 3,
            paused: false,
            pending: 0.0,
        })
        .insert_resource(options.clone())
        .insert_resource(Walk { cooldown: 0.0 })
        .insert_resource(SmellView(smell))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (controls, walk, advance, draw_tiles, draw_animals, draw_panel, screenshot).chain(),
        )
        .run();
}

/// Opções da linha de comandos.
#[derive(Resource, Clone)]
struct Options {
    seed: u64,
    cabin: bool,
    /// Começar com a vista do cheiro ligada.
    smell: bool,
    /// Guardar uma imagem da janela e sair (útil para o diário e para testes).
    screenshot: Option<String>,
}

impl Options {
    fn from_args() -> Self {
        let mut o = Options {
            seed: 42,
            cabin: false,
            smell: false,
            screenshot: None,
        };
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--seed" => {
                    i += 1;
                    o.seed = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(o.seed);
                }
                "--cabin" => o.cabin = true,
                "--smell" => o.smell = true,
                "--screenshot" => {
                    i += 1;
                    o.screenshot = args.get(i).cloned();
                }
                other => eprintln!("Argumento desconhecido (ignorado): {other}"),
            }
            i += 1;
        }
        o
    }
}

/// A simulação, guardada como recurso do Bevy.
#[derive(Resource)]
struct World(Simulation);

/// O ritmo do tempo de jogo.
#[derive(Resource)]
struct Clock {
    /// Índice em `SPEEDS`.
    speed: usize,
    paused: bool,
    /// Fração de tick que ainda falta correr (o tempo real não chega em ticks certos).
    pending: f32,
}

/// O ritmo a que o jogador anda com uma tecla carregada.
#[derive(Resource)]
struct Walk {
    /// Segundos até poder dar o próximo passo.
    cooldown: f32,
}

/// A vista do cheiro (tecla O): pinta o rasto do jogador a azul e o cheiro
/// de comida a amarelo por cima da vegetação (GDD 20.12, ver os campos de cheiro).
#[derive(Resource)]
struct SmellView(bool);

/// Uma célula do mapa (o índice é `y * largura + x`).
#[derive(Component)]
struct Tile(usize);

/// Um coelho, uma raposa ou o jogador, desenhado neste frame.
#[derive(Component)]
struct AnimalSprite;

/// O texto com o dia, as populações e as teclas.
#[derive(Component)]
struct Panel;

/// O centro de uma célula no ecrã. A linha 0 do mapa fica em cima.
fn cell_to_screen(x: usize, y: usize, width: usize, height: usize) -> Vec2 {
    Vec2::new(
        (x as f32 + 0.5 - width as f32 / 2.0) * CELL,
        (height as f32 / 2.0 - y as f32 - 0.5) * CELL,
    )
}

/// A cor da vegetação, de terra nua (castanho) a erva cheia (verde).
fn vegetation_color(ratio: f32) -> Color {
    let r = ratio.clamp(0.0, 1.0);
    let bare = Vec3::new(0.42, 0.33, 0.22);
    let full = Vec3::new(0.16, 0.50, 0.18);
    let c = bare.lerp(full, r);
    Color::srgb(c.x, c.y, c.z)
}

fn setup(mut commands: Commands, world: Res<World>, assets: Res<AssetServer>) {
    // A câmara desvia-se para a esquerda, para o mapa ficar à direita do painel.
    commands.spawn((Camera2d, Transform::from_xyz(-PANEL_WIDTH / 2.0, 0.0, 0.0)));
    let sim = &world.0;
    let (w, h) = (sim.world.width, sim.world.height);
    for y in 0..h {
        for x in 0..w {
            let pos = cell_to_screen(x, y, w, h);
            // Um píxel de folga entre células desenha uma grelha discreta.
            commands.spawn((
                Sprite::from_color(Color::BLACK, Vec2::splat(CELL - 1.0)),
                Transform::from_xyz(pos.x, pos.y, 0.0),
                Tile(y * w + x),
            ));
            if sim.world.is_burrow(x, y) {
                commands.spawn((
                    Sprite::from_color(Color::srgb(0.20, 0.13, 0.08), Vec2::splat(CELL * 0.45)),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                ));
            }
        }
    }
    if let Some(c) = &sim.config().cabin {
        let pos = cell_to_screen(c.x, c.y, w, h);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.55, 0.38, 0.22), Vec2::splat(CELL * 0.9)),
            Transform::from_xyz(pos.x, pos.y, 1.5),
        ));
    }
    // A fonte do Bevy não tem acentos ("Vegetação" aparecia com quadrados).
    // A DejaVu Sans tem, e a licença deixa-a ir com o jogo (ver assets/fonts).
    commands.spawn((
        Text::new(""),
        TextFont {
            font: assets.load("fonts/DejaVuSans.ttf"),
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.92, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(14.0),
            width: Val::Px(PANEL_WIDTH - 28.0),
            ..default()
        },
        Panel,
    ));
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<Clock>,
    mut smell: ResMut<SmellView>,
) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if keys.just_pressed(KeyCode::KeyO) {
        smell.0 = !smell.0;
    }
    if keys.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd]) {
        clock.speed = (clock.speed + 1).min(SPEEDS.len() - 1);
    }
    if keys.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract]) {
        clock.speed = clock.speed.saturating_sub(1);
    }
}

/// Move o jogador com as setas ou WASD: um passo logo que se carrega, e
/// depois um passo a cada `PLAYER_STEP_SECS` enquanto a tecla estiver em baixo.
/// Anda em tempo real, mesmo com o jogo em pausa.
fn walk(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut walk: ResMut<Walk>,
    mut world: ResMut<World>,
) {
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    let pressed = |a, b| keys.just_pressed(a) || keys.just_pressed(b);
    let dirs = [
        (KeyCode::ArrowUp, KeyCode::KeyW, 0, -1),
        (KeyCode::ArrowDown, KeyCode::KeyS, 0, 1),
        (KeyCode::ArrowLeft, KeyCode::KeyA, -1, 0),
        (KeyCode::ArrowRight, KeyCode::KeyD, 1, 0),
    ];
    let (mut dx, mut dy): (i32, i32) = (0, 0);
    let mut fresh = false;
    for (a, b, x, y) in dirs {
        if held(a, b) {
            dx += x;
            dy += y;
        }
        fresh |= pressed(a, b);
    }
    walk.cooldown -= time.delta_secs();
    if (dx, dy) == (0, 0) {
        return;
    }
    if fresh || walk.cooldown <= 0.0 {
        world.0.move_player(dx.signum(), dy.signum());
        walk.cooldown = PLAYER_STEP_SECS;
    }
}

/// Corre os ticks que couberem no tempo real que passou.
fn advance(time: Res<Time>, mut clock: ResMut<Clock>, mut world: ResMut<World>) {
    if clock.paused {
        return;
    }
    clock.pending += time.delta_secs() * SPEEDS[clock.speed];
    // Um limite por frame, para um frame lento não congelar a janela.
    let ticks = (clock.pending as u32).min(240);
    clock.pending -= ticks as f32;
    for _ in 0..ticks {
        world.0.step();
    }
}

fn draw_tiles(world: Res<World>, smell: Res<SmellView>, mut tiles: Query<(&Tile, &mut Sprite)>) {
    let w = &world.0.world;
    for (tile, mut sprite) in &mut tiles {
        let (x, y) = (tile.0 % w.width, tile.0 / w.width);
        let mut color = vegetation_color(w.biomass_at(x, y) / w.capacity());
        if smell.0 {
            let human = w.scent.at(Scent::Human, x, y);
            let food = w.scent.at(Scent::Food, x, y);
            color = smell_tint(color, human, food);
        }
        sprite.color = color;
    }
}

/// Mistura a cor da célula com azul (cheiro humano) e amarelo (comida).
/// O azul fica forte a partir de 1, perto do valor que já afasta as raposas.
fn smell_tint(base: Color, human: f32, food: f32) -> Color {
    let c = base.to_srgba();
    let mut rgb = Vec3::new(c.red, c.green, c.blue);
    rgb = rgb.lerp(Vec3::new(0.95, 0.85, 0.20), (food / 5.0).clamp(0.0, 0.85));
    rgb = rgb.lerp(Vec3::new(0.25, 0.45, 1.0), human.clamp(0.0, 0.85));
    Color::srgb(rgb.x, rgb.y, rgb.z)
}

/// Apaga os animais do frame anterior e desenha os de agora. São poucas
/// centenas, por isso não vale a pena complicar.
fn draw_animals(
    mut commands: Commands,
    world: Res<World>,
    old: Query<Entity, With<AnimalSprite>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let sim = &world.0;
    let (w, h) = (sim.world.width, sim.world.height);
    for (i, r) in sim.rabbits.iter().enumerate() {
        // Vários coelhos na mesma célula ficam ligeiramente desencontrados.
        let jitter = Vec2::new((i % 3) as f32 - 1.0, ((i / 3) % 3) as f32 - 1.0) * CELL * 0.22;
        let pos = cell_to_screen(r.x, r.y, w, h) + jitter;
        commands.spawn((
            Sprite::from_color(Color::srgb(0.93, 0.90, 0.82), Vec2::splat(CELL * 0.28)),
            Transform::from_xyz(pos.x, pos.y, 2.0),
            AnimalSprite,
        ));
    }
    for f in &sim.foxes {
        let pos = cell_to_screen(f.x, f.y, w, h);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.90, 0.45, 0.12), Vec2::splat(CELL * 0.6)),
            Transform::from_xyz(pos.x, pos.y, 3.0),
            AnimalSprite,
        ));
    }
    if let Some((x, y)) = sim.player {
        // O jogador: um quadrado azul com contorno claro, para se ver bem.
        let pos = cell_to_screen(x, y, w, h);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.95, 0.95, 1.0), Vec2::splat(CELL * 0.8)),
            Transform::from_xyz(pos.x, pos.y, 4.0),
            AnimalSprite,
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb(0.20, 0.45, 0.95), Vec2::splat(CELL * 0.62)),
            Transform::from_xyz(pos.x, pos.y, 4.1),
            AnimalSprite,
        ));
    }
}

fn draw_panel(
    world: Res<World>,
    clock: Res<Clock>,
    smell: Res<SmellView>,
    mut panel: Query<&mut Text, With<Panel>>,
) {
    let sim = &world.0;
    let Ok(mut text) = panel.single_mut() else {
        return;
    };
    let state = if clock.paused { "em pausa" } else { "a correr" };
    let cabin = if sim.config().cabin.is_some() {
        "\n  castanho claro: cabana"
    } else {
        ""
    };
    // As últimas linhas do registo da cabana, se houver cabana.
    let log = sim.cabin_log.lines.iter().rev().take(3).rev();
    let log: String = log.map(|l| format!("\n{l}")).collect();
    text.0 = format!(
        "Echoes of the Wild\n\n\
         Dia {}, {:02}h\n{state}, {} horas por segundo\n\n\
         Coelhos: {}\nRaposas: {}\nVegetação média: {:.1} de {:.0}\n\n\
         Setas ou WASD: andar\nEspaço: pausa\n+ e -: velocidade\nO: ver o cheiro ({})\n\n\
         Legenda:\n  quadrado azul: tu\n  branco: coelho\n  laranja: raposa\n  castanho escuro: toca{cabin}\n  \
         verde a castanho: muita a pouca erva{}",
        sim.day(),
        sim.tick() % TICKS_PER_DAY,
        SPEEDS[clock.speed],
        sim.rabbits.len(),
        sim.foxes.len(),
        sim.world.mean_biomass(),
        sim.world.capacity(),
        if smell.0 { "ligado: azul é o teu rasto, amarelo a comida" } else { "desligado" },
        if log.is_empty() { String::new() } else { format!("\n\nCabana:{log}") },
    );
}

/// Com `--screenshot`, espera uns frames (para a janela estar desenhada),
/// guarda a imagem e fecha.
fn screenshot(
    mut commands: Commands,
    options: Res<Options>,
    mut frames: Local<u32>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(path) = &options.screenshot else {
        return;
    };
    *frames += 1;
    if *frames == 30 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if *frames == 60 {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mais_vegetacao_e_mais_verde() {
        let pouca = vegetation_color(0.1).to_srgba();
        let muita = vegetation_color(0.9).to_srgba();
        assert!(muita.green > pouca.green);
        assert!(muita.red < pouca.red);
    }

    /// Corre o sistema `walk` uma vez com estas teclas carregadas e devolve
    /// onde fica o jogador, que começa em (5, 5).
    fn andar_com(teclas: &[KeyCode]) -> (usize, usize) {
        let mut sim = Simulation::new(Config::default());
        sim.place_player(Some((5, 5)));
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        for &k in teclas {
            input.press(k);
        }
        app.insert_resource(input)
            .insert_resource(Time::<()>::default())
            .insert_resource(Walk { cooldown: 0.0 })
            .insert_resource(World(sim))
            .add_systems(Update, walk);
        app.update();
        app.world().resource::<World>().0.player.unwrap()
    }

    #[test]
    fn as_teclas_mexem_o_jogador() {
        assert_eq!(andar_com(&[KeyCode::KeyW]), (5, 4));
        assert_eq!(andar_com(&[KeyCode::ArrowDown]), (5, 6));
        assert_eq!(andar_com(&[KeyCode::KeyA]), (4, 5));
        assert_eq!(andar_com(&[KeyCode::ArrowRight]), (6, 5));
        // Duas teclas ao mesmo tempo: diagonal.
        assert_eq!(andar_com(&[KeyCode::KeyW, KeyCode::KeyA]), (4, 4));
        assert_eq!(andar_com(&[KeyCode::ArrowDown, KeyCode::ArrowRight]), (6, 6));
        // Teclas opostas anulam-se.
        assert_eq!(andar_com(&[KeyCode::KeyW, KeyCode::KeyS]), (5, 5));
    }

    #[test]
    fn a_tecla_o_liga_e_desliga_o_cheiro() {
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyO);
        app.insert_resource(input)
            .insert_resource(Clock {
                speed: 3,
                paused: false,
                pending: 0.0,
            })
            .insert_resource(SmellView(false))
            .add_systems(Update, controls);
        app.update();
        assert!(app.world().resource::<SmellView>().0, "devia ter ligado");
    }

    /// O jogador a andar à velocidade normal do jogo (cerca de 8 células por
    /// segundo, com o tempo a 24 horas por segundo) deixa um rasto que se vê.
    #[test]
    fn o_rasto_de_quem_anda_fica_azul() {
        let mut sim = Simulation::new(Config::default());
        // Em cada tick o jogador anda 1 ou 0 células (8 células por 3 ticks).
        for t in 0..24usize {
            sim.place_player(Some((2 + t / 3, 10)));
            sim.step();
        }
        let base = vegetation_color(0.5).to_srgba();
        for x in 2..9 {
            let human = sim.world.scent.at(Scent::Human, x, 10);
            let tinted = smell_tint(vegetation_color(0.5), human, 0.0).to_srgba();
            assert!(tinted.blue > base.blue + 0.3, "célula ({x}, 10): cheiro {human:.2}, pouco azul");
        }
    }

    #[test]
    fn o_cheiro_humano_pinta_de_azul() {
        let base = vegetation_color(0.5);
        let sem = smell_tint(base, 0.0, 0.0).to_srgba();
        let com = smell_tint(base, 1.0, 0.0).to_srgba();
        assert_eq!(sem, base.to_srgba(), "sem cheiro, a cor não muda");
        assert!(com.blue > sem.blue + 0.3);
    }

    #[test]
    fn a_linha_zero_fica_em_cima() {
        let topo = cell_to_screen(0, 0, 24, 24);
        let fundo = cell_to_screen(0, 23, 24, 24);
        assert!(topo.y > fundo.y);
        // O mapa fica centrado na origem.
        assert_eq!(cell_to_screen(0, 0, 24, 24).x, -cell_to_screen(23, 0, 24, 24).x);
    }
}
