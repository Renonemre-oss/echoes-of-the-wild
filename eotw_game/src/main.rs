//! O jogo: a paisagem do `eotw_sim` desenhada numa janela com Bevy.
//!
//! A simulação não sabe que o Bevy existe. Este programa só faz três coisas:
//! avança a simulação ao ritmo escolhido, desenha o que ela calculou e lê o
//! teclado. Toda a ecologia continua em `eotw_sim`.
//!
//! O mundo é uma paisagem de 5x5 regiões (120x120 células). A câmara segue
//! o jogador e só se desenha o que está à volta dele, que está sempre dentro
//! da janela simulada por inteiro (`Landscape`).
//!
//! Exemplos:
//!   cargo run -p eotw_game
//!   cargo run -p eotw_game -- --seed 7 --cabin
//!   cargo run -p eotw_game -- --screenshot janela.png
//!   cargo run -p eotw_game -- --smell   (começa com a vista do cheiro ligada)
//!   cargo run --release -p eotw_game -- --frames 600   (mede o tempo por frame e sai)
//!   cargo run --release -p eotw_game -- --speed 240     (começa a 240 horas por segundo)

use bevy::prelude::*;
use bevy::render::settings::{PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::RenderPlugin;
use eotw_sim::landscape::{Landscape, REGION};
use eotw_sim::{CabinParams, Config, Scent, TICKS_PER_DAY};

/// Tamanho de uma célula do mapa no ecrã, em píxeis.
const CELL: f32 = 26.0;

/// Largura reservada ao painel de texto, à esquerda do mapa.
const PANEL_WIDTH: f32 = 330.0;

/// Regiões de lado na paisagem.
const LANDSCAPE_REGIONS: usize = 5;

/// Células desenhadas à volta do jogador, de lado a lado e de cima a baixo.
/// Chegam para encher a parte da janela à direita do painel (630 x 720
/// píxeis, 25 x 28 células) e ficam dentro da janela simulada, que vai pelo
/// menos 24 células para cada lado do jogador.
const VIEW_W: usize = 27;
const VIEW_H: usize = 31;
const _: () = assert!(VIEW_W / 2 <= REGION && VIEW_H / 2 <= REGION);

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
    let side = LANDSCAPE_REGIONS * REGION;
    if options.cabin {
        config.cabin = Some(CabinParams::centered(side, side, 10.0));
    }
    // O jogador chega depois do aquecimento, a um mundo já em equilíbrio,
    // ao lado da cabana (ou ao centro, se não houver cabana).
    let start = config.cabin.as_ref().map_or((side / 2, side / 2), |c| (c.x, c.y + 1));
    let land = Landscape::new(config, LANDSCAPE_REGIONS, LANDSCAPE_REGIONS, start);

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(render_plugin())
                .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Echoes of the Wild".into(),
                resolution: (960.0_f32, 720.0_f32).into(),
                // A medir, sem esperar pelo ecrã: vê-se quanto o frame custa a sério.
                present_mode: if options.frames.is_some() {
                    bevy::window::PresentMode::AutoNoVsync
                } else {
                    bevy::window::PresentMode::AutoVsync
                },
                ..default()
            }),
            ..default()
        }),
        )
        .insert_resource(ClearColor(Color::srgb(0.10, 0.11, 0.10)))
        .insert_resource(World(land))
        .insert_resource(Clock {
            // O ritmo de `SPEEDS` mais perto do pedido com `--speed`.
            speed: (0..SPEEDS.len())
                .min_by(|&a, &b| (SPEEDS[a] - options.speed).abs().total_cmp(&(SPEEDS[b] - options.speed).abs()))
                .unwrap_or(3),
            paused: false,
            pending: 0.0,
        })
        .insert_resource(options.clone())
        .insert_resource(Walk { cooldown: 0.0 })
        .insert_resource(SmellView(smell))
        .init_resource::<Perf>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (controls, walk, advance, follow, draw_tiles, draw_animals, draw_panel, screenshot, measure).chain(),
        )
        .run();
}

/// A placa gráfica a usar. Num portátil com duas, a integrada por omissão:
/// para um jogo 2D chega bem, e neste portátil a NVIDIA (MX250 com o
/// controlador NVK) dava um soluço de 33 ms a cada 20 frames, mesmo sem nada
/// para desenhar. A Intel integrada fica abaixo de 7 ms no pior frame. Com
/// `WGPU_POWER_PREF=high` usa-se a outra.
fn render_plugin() -> RenderPlugin {
    let mut settings = WgpuSettings::default();
    if std::env::var("WGPU_POWER_PREF").is_err() {
        settings.power_preference = PowerPreference::LowPower;
    }
    RenderPlugin {
        render_creation: RenderCreation::Automatic(settings),
        ..default()
    }
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
    /// Correr este número de frames, escrever quanto demoraram e sair.
    frames: Option<u32>,
    /// Horas de jogo por segundo ao começar (uma das de `SPEEDS`).
    speed: f32,
}

impl Options {
    fn from_args() -> Self {
        let mut o = Options {
            seed: 42,
            cabin: false,
            smell: false,
            screenshot: None,
            frames: None,
            speed: 24.0,
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
                "--speed" => {
                    i += 1;
                    o.speed = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(o.speed);
                }
                "--frames" => {
                    i += 1;
                    o.frames = args.get(i).and_then(|v| v.parse().ok());
                }
                other => eprintln!("Argumento desconhecido (ignorado): {other}"),
            }
            i += 1;
        }
        o
    }
}

/// A paisagem, guardada como recurso do Bevy.
#[derive(Resource)]
struct World(Landscape);

/// O ritmo do tempo de jogo.
#[derive(Resource)]
struct Clock {
    /// Índice em `SPEEDS`.
    speed: usize,
    paused: bool,
    /// Fração de tick que ainda falta correr (o tempo real não chega em ticks certos).
    pending: f32,
}

/// Quanto tempo a simulação gastou no último frame (para o painel e `--frames`).
#[derive(Resource, Default)]
struct Perf {
    sim_ms: f32,
    /// O que o painel mostra: frames por segundo e o pior tempo da
    /// simulação, renovados duas vezes por segundo (mudar o texto em cada
    /// frame também custa).
    fps: f32,
    sim_peak: f32,
    frames: u32,
    elapsed: f32,
    peak: f32,
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

/// Um dos quadrados desenhados à volta do jogador. O índice diz a posição
/// em relação a ele: coluna `i % VIEW_W` e linha `i / VIEW_W` da vista.
#[derive(Component)]
struct Tile(usize);

/// Um coelho, uma raposa ou o jogador, desenhado neste frame.
#[derive(Component)]
struct AnimalSprite;

/// O texto com o dia, as populações e as teclas.
#[derive(Component)]
struct Panel;

/// O centro de uma célula da paisagem no ecrã. A linha 0 fica em cima, e
/// a câmara é que se mexe para seguir o jogador.
fn cell_to_screen(x: i64, y: i64) -> Vec2 {
    Vec2::new((x as f32 + 0.5) * CELL, -(y as f32 + 0.5) * CELL)
}

/// A célula da paisagem por baixo do quadrado `i` da vista, com o jogador ao centro.
fn tile_cell(i: usize, player: (usize, usize)) -> (i64, i64) {
    (
        player.0 as i64 + (i % VIEW_W) as i64 - (VIEW_W / 2) as i64,
        player.1 as i64 + (i / VIEW_W) as i64 - (VIEW_H / 2) as i64,
    )
}

/// A célula está à vista: a menos de meia vista do jogador.
fn on_screen((x, y): (i64, i64), player: (usize, usize)) -> bool {
    (x - player.0 as i64).unsigned_abs() as usize <= VIEW_W / 2
        && (y - player.1 as i64).unsigned_abs() as usize <= VIEW_H / 2
}

/// Uma célula da paisagem em coordenadas da janela simulada, se estiver nela.
fn in_window(land: &Landscape, (x, y): (i64, i64)) -> Option<(usize, usize)> {
    if x < 0 || y < 0 {
        return None;
    }
    land.in_window((x as usize, y as usize))
}

/// A cor da vegetação, de terra nua (castanho) a erva cheia (verde).
fn vegetation_color(ratio: f32) -> Color {
    let r = ratio.clamp(0.0, 1.0);
    let bare = Vec3::new(0.42, 0.33, 0.22);
    let full = Vec3::new(0.16, 0.50, 0.18);
    let c = bare.lerp(full, r);
    Color::srgb(c.x, c.y, c.z)
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2d);
    // Os quadrados da vista: sempre os mesmos, e em cada frame mudam de
    // sítio e de cor para mostrar o que está à volta do jogador.
    for i in 0..VIEW_W * VIEW_H {
        // Um píxel de folga entre células desenha uma grelha discreta.
        commands.spawn((
            Sprite::from_color(Color::BLACK, Vec2::splat(CELL - 1.0)),
            Transform::default(),
            Tile(i),
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
fn advance(time: Res<Time>, mut clock: ResMut<Clock>, mut world: ResMut<World>, mut perf: ResMut<Perf>) {
    let start = std::time::Instant::now();
    if clock.paused {
        perf.sim_ms = 0.0;
        return;
    }
    clock.pending += time.delta_secs() * SPEEDS[clock.speed];
    // Um limite por frame, para um frame lento não congelar a janela.
    let ticks = (clock.pending as u32).min(240);
    clock.pending -= ticks as f32;
    for _ in 0..ticks {
        world.0.step();
    }
    perf.sim_ms = start.elapsed().as_secs_f32() * 1000.0;
}

/// A câmara segue o jogador. Fica desviada para a esquerda, para o jogador
/// aparecer no meio da parte do ecrã à direita do painel.
fn follow(world: Res<World>, mut camera: Query<&mut Transform, With<Camera2d>>) {
    let (x, y) = world.0.player();
    let at = cell_to_screen(x as i64, y as i64);
    for mut t in &mut camera {
        t.translation.x = at.x - PANEL_WIDTH / 2.0;
        t.translation.y = at.y;
    }
}

fn draw_tiles(
    world: Res<World>,
    smell: Res<SmellView>,
    mut tiles: Query<(&Tile, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let land = &world.0;
    let w = &land.window_sim().world;
    let player = land.player();
    for (tile, mut sprite, mut transform, mut visible) in &mut tiles {
        let cell = tile_cell(tile.0, player);
        let pos = cell_to_screen(cell.0, cell.1);
        // Só se mexe no que mudou: cada alteração obriga o Bevy a voltar a
        // enviar o sprite para a placa gráfica.
        transform.set_if_neq(Transform::from_translation(pos.extend(0.0)));
        // Fora da paisagem não há nada para desenhar.
        let Some((x, y)) = in_window(land, cell) else {
            visible.set_if_neq(Visibility::Hidden);
            continue;
        };
        visible.set_if_neq(Visibility::Visible);
        let mut color = vegetation_color(w.biomass_at(x, y) / w.capacity());
        if smell.0 {
            let human = w.scent.at(Scent::Human, x, y);
            let food = w.scent.at(Scent::Food, x, y);
            color = smell_tint(color, human, food);
        }
        if sprite.color != color {
            sprite.color = color;
        }
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

/// Uma coisa a desenhar por cima dos quadrados: em que célula, com que
/// desvio dentro dela, cor, tamanho e camada.
struct Mark {
    cell: (i64, i64),
    offset: Vec2,
    color: Color,
    size: f32,
    z: f32,
}

/// Uma marca no centro da célula.
fn mark(cell: (i64, i64), color: Color, size: f32, z: f32) -> Mark {
    Mark { cell, offset: Vec2::ZERO, color, size, z }
}

/// O que se desenha por cima dos quadrados (tocas, cabana, animais,
/// jogador): os sprites são sempre os mesmos, e em cada frame mudam de
/// sítio, cor e tamanho; os que sobram escondem-se. Antes, cada frame
/// apagava e criava uma entidade por cada coisa à vista, e isso, com
/// centenas de coelhos, levava a maior parte do frame (22 ms contra 5).
fn draw_animals(
    mut commands: Commands,
    world: Res<World>,
    mut pool: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<AnimalSprite>>,
) {
    let land = &world.0;
    let sim = land.window_sim();
    let (ox, oy) = land.window_origin();
    let player = land.player();
    let global = |x: usize, y: usize| ((ox + x) as i64, (oy + y) as i64);
    let mut items: Vec<Mark> = Vec::new();
    for &(x, y) in sim.world.burrows() {
        items.push(mark(global(x, y), Color::srgb(0.20, 0.13, 0.08), CELL * 0.45, 1.0));
    }
    if let Some(c) = land.cabin() {
        items.push(mark((c.x as i64, c.y as i64), Color::srgb(0.55, 0.38, 0.22), CELL * 0.9, 1.5));
    }
    for (i, r) in sim.rabbits.iter().enumerate() {
        // Vários coelhos na mesma célula ficam ligeiramente desencontrados.
        let jitter = Vec2::new((i % 3) as f32 - 1.0, ((i / 3) % 3) as f32 - 1.0) * CELL * 0.22;
        items.push(Mark {
            offset: jitter,
            ..mark(global(r.x, r.y), Color::srgb(0.93, 0.90, 0.82), CELL * 0.28, 2.0)
        });
    }
    for f in &sim.foxes {
        items.push(mark(global(f.x, f.y), Color::srgb(0.90, 0.45, 0.12), CELL * 0.6, 3.0));
    }
    // O jogador: um quadrado azul com contorno claro, para se ver bem.
    let me = (player.0 as i64, player.1 as i64);
    items.push(mark(me, Color::srgb(0.95, 0.95, 1.0), CELL * 0.8, 4.0));
    items.push(mark(me, Color::srgb(0.20, 0.45, 0.95), CELL * 0.62, 4.1));
    items.retain(|m| on_screen(m.cell, player));

    let mut sprites = pool.iter_mut();
    for &Mark { cell, offset, color, size, z } in &items {
        let pos = (cell_to_screen(cell.0, cell.1) + offset).extend(z);
        match sprites.next() {
            Some((mut sprite, mut transform, mut visible)) => {
                let size = Some(Vec2::splat(size));
                if sprite.color != color || sprite.custom_size != size {
                    sprite.color = color;
                    sprite.custom_size = size;
                }
                transform.set_if_neq(Transform::from_translation(pos));
                visible.set_if_neq(Visibility::Visible);
            }
            // Faltam sprites: criam-se mais (aparecem no frame seguinte).
            None => {
                commands.spawn((Sprite::from_color(color, Vec2::splat(size)), Transform::from_translation(pos), AnimalSprite));
            }
        }
    }
    for (_, _, mut visible) in sprites {
        visible.set_if_neq(Visibility::Hidden);
    }
}

fn draw_panel(
    world: Res<World>,
    clock: Res<Clock>,
    smell: Res<SmellView>,
    perf: Res<Perf>,
    mut panel: Query<&mut Text, With<Panel>>,
) {
    let land = &world.0;
    let sim = land.window_sim();
    let Ok(mut text) = panel.single_mut() else {
        return;
    };
    let state = if clock.paused { "em pausa" } else { "a correr" };
    let cabin = if land.cabin().is_some() {
        "\n  castanho claro: cabana"
    } else {
        ""
    };
    let totals = land.totals();
    // As últimas linhas do registo da cabana, se houver cabana.
    let log = sim.cabin_log.lines.iter().rev().take(3).rev();
    let log: String = log.map(|l| format!("\n{l}")).collect();
    let new = format!(
        "Echoes of the Wild\n\n\
         Dia {}, {:02}h\n{state}, {} horas por segundo\n\
         {:.0} frames por segundo, simulação até {:.1} ms\n\n\
         Região {}, {} (de {} x {})\n\
         À volta (3x3 regiões): {} coelhos, {} raposas\n\
         No mundo: {} coelhos, {} raposas\n\
         Vegetação média à volta: {:.1} de {:.0}\n\n\
         Setas ou WASD: andar\nEspaço: pausa\n+ e -: velocidade\nO: ver o cheiro ({})\n\n\
         Legenda:\n  quadrado azul: tu\n  branco: coelho\n  laranja: raposa\n  castanho escuro: toca{cabin}\n  \
         verde a castanho: muita a pouca erva{}",
        sim.day(),
        sim.tick() % TICKS_PER_DAY,
        SPEEDS[clock.speed],
        perf.fps,
        perf.sim_peak,
        land.player().0 / REGION,
        land.player().1 / REGION,
        land.cols,
        land.rows,
        sim.rabbits.len(),
        sim.foxes.len(),
        totals.0,
        totals.1,
        sim.world.mean_biomass(),
        sim.world.capacity(),
        if smell.0 { "ligado: azul é o teu rasto, amarelo a comida" } else { "desligado" },
        if log.is_empty() { String::new() } else { format!("\n\nCabana:{log}") },
    );
    // Refazer o texto obriga a voltar a compor as letras: só quando muda.
    if text.0 != new {
        text.0 = new;
    }
}

/// Conta os frames por segundo para o painel. Com `--frames N`, também
/// conta o tempo de cada frame e, ao fim de N, escreve a
/// mediana e o pior e sai. Os primeiros 30 frames não contam (a janela ainda
/// está a abrir). `sim` é o tempo gasto a avançar a simulação.
fn measure(
    options: Res<Options>,
    time: Res<Time<Real>>,
    mut perf: ResMut<Perf>,
    mut samples: Local<Vec<(f32, f32)>>,
    mut exit: EventWriter<AppExit>,
) {
    perf.frames += 1;
    perf.elapsed += time.delta_secs();
    perf.peak = perf.peak.max(perf.sim_ms);
    if perf.elapsed >= 0.5 {
        perf.fps = perf.frames as f32 / perf.elapsed;
        perf.sim_peak = perf.peak;
        (perf.frames, perf.elapsed, perf.peak) = (0, 0.0, 0.0);
    }
    let Some(n) = options.frames else {
        return;
    };
    samples.push((time.delta_secs() * 1000.0, perf.sim_ms));
    if samples.len() as u32 == n + 30 {
        let mut frame: Vec<f32> = samples[30..].iter().map(|s| s.0).collect();
        let mut sim: Vec<f32> = samples[30..].iter().map(|s| s.1).collect();
        frame.sort_by(f32::total_cmp);
        sim.sort_by(f32::total_cmp);
        let at = |v: &[f32], q: f32| v[((v.len() - 1) as f32 * q) as usize];
        println!(
            "{n} frames: frame mediana {:.1} ms ({:.0} fps), p95 {:.1} ms, pior {:.1} ms | simulação mediana {:.2} ms, pior {:.2} ms",
            at(&frame, 0.5),
            1000.0 / at(&frame, 0.5),
            at(&frame, 0.95),
            at(&frame, 1.0),
            at(&sim, 0.5),
            at(&sim, 1.0)
        );
        exit.write(AppExit::Success);
    }
}

/// Com `--screenshot`, espera 2 segundos (para a janela estar desenhada e
/// a letra carregada), guarda a imagem e fecha. Contar frames não chegava:
/// em release, 30 frames passavam antes de a letra do painel carregar.
fn screenshot(
    mut commands: Commands,
    options: Res<Options>,
    time: Res<Time<Real>>,
    mut taken: Local<Option<f32>>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(path) = &options.screenshot else {
        return;
    };
    let now = time.elapsed_secs();
    match *taken {
        None if now >= 2.0 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            *taken = Some(now);
        }
        Some(at) if now >= at + 1.0 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eotw_sim::Simulation;

    /// Uma paisagem pequena (1 região), para os testes serem rápidos.
    fn paisagem(jogador: (usize, usize)) -> Landscape {
        Landscape::new(Config::default(), 1, 1, jogador)
    }

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
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        for &k in teclas {
            input.press(k);
        }
        app.insert_resource(input)
            .insert_resource(Time::<()>::default())
            .insert_resource(Walk { cooldown: 0.0 })
            .insert_resource(World(paisagem((5, 5))))
            .add_systems(Update, walk);
        app.update();
        app.world().resource::<World>().0.player()
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
        assert!(cell_to_screen(0, 0).y > cell_to_screen(0, 23).y);
        assert!(cell_to_screen(0, 0).x < cell_to_screen(1, 0).x);
    }

    /// O que se desenha à volta do jogador enche o ecrã.
    #[test]
    fn a_vista_enche_o_ecra() {
        // (que a vista cabe na janela é verificado ao compilar, junto a `VIEW_W`)
        // A vista enche a parte do ecrã à direita do painel (960 x 720).
        assert!(VIEW_W as f32 * CELL >= 960.0 - PANEL_WIDTH);
        assert!(VIEW_H as f32 * CELL >= 720.0);
    }

    /// A câmara fica em cima do jogador (desviada para o painel caber à
    /// esquerda), também depois de ele andar.
    #[test]
    fn a_camara_segue_o_jogador() {
        let mut app = App::new();
        app.insert_resource(World(paisagem((5, 5)))).add_systems(Update, follow);
        let camera = app.world_mut().spawn((Camera2d, Transform::default())).id();
        for passo in 0..3 {
            app.world_mut().resource_mut::<World>().0.move_player(1, 0);
            app.update();
            let t = app.world().get::<Transform>(camera).unwrap().translation;
            let jogador = cell_to_screen(6 + passo, 5);
            assert_eq!((t.x, t.y), (jogador.x - PANEL_WIDTH / 2.0, jogador.y));
        }
    }

    /// Os sprites dos animais reaproveitam-se: depois do primeiro frame não
    /// se criam mais, e há um visível por cada coisa à vista.
    #[test]
    fn os_sprites_dos_animais_reaproveitam_se() {
        let mut app = App::new();
        app.insert_resource(World(paisagem((12, 12)))).add_systems(Update, draw_animals);
        let contar = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<&Visibility, With<AnimalSprite>>();
            let todos: Vec<Visibility> = q.iter(app.world()).copied().collect();
            (todos.len(), todos.iter().filter(|v| **v == Visibility::Visible).count())
        };
        app.update(); // cria os sprites
        app.update(); // usa-os
        let (criados, visiveis) = contar(&mut app);
        assert!(visiveis > 2, "devia haver animais à vista, além do jogador");
        assert_eq!(criados, visiveis);
        // A simulação avança e os animais mudam: os sprites continuam os mesmos.
        app.world_mut().resource_mut::<World>().0.run_days(2);
        app.update();
        app.update();
        let (depois, visiveis) = contar(&mut app);
        assert!(depois <= criados.max(visiveis), "criou sprites a mais: {criados} → {depois}");
        assert!(depois >= visiveis);
    }

    /// Os quadrados mostram a vegetação da célula por baixo, com o jogador
    /// ao centro, e escondem-se fora da paisagem.
    #[test]
    fn os_quadrados_mostram_o_mundo_a_volta() {
        let mut app = App::new();
        app.insert_resource(World(paisagem((2, 3))))
            .insert_resource(SmellView(false))
            .add_systems(Update, draw_tiles);
        let centro = (VIEW_H / 2) * VIEW_W + VIEW_W / 2;
        let canto = 0; // em cima à esquerda: fora da paisagem
        let ids: Vec<Entity> = [centro, canto]
            .into_iter()
            .map(|i| {
                app.world_mut()
                    .spawn((Sprite::default(), Transform::default(), Visibility::default(), Tile(i)))
                    .id()
            })
            .collect();
        app.update();
        let land = &app.world().resource::<World>().0;
        let w = &land.window_sim().world;
        let esperada = vegetation_color(w.biomass_at(2, 3) / w.capacity());
        assert_eq!(app.world().get::<Sprite>(ids[0]).unwrap().color, esperada);
        assert_eq!(app.world().get::<Transform>(ids[0]).unwrap().translation.truncate(), cell_to_screen(2, 3));
        assert_eq!(*app.world().get::<Visibility>(ids[0]).unwrap(), Visibility::Visible);
        assert_eq!(*app.world().get::<Visibility>(ids[1]).unwrap(), Visibility::Hidden);
    }
}
