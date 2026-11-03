//! Run with `cargo run -p bear-cub-berries` (add `--release` for a smooth frame rate).
//!
//! **Bear Cub Berries** — a tiny roguelite about feeding the den before the
//! first snow.
//!
//! You are a bear cub. Winter is close, and the den is hungry: old Mama can no
//! longer leave the hollow, and your little brother Pip is too sick to walk.
//! They talk, they worry, they tease you, and they love honey. But they are
//! still bears: four paws on the ground, a nose that knows berries, and the
//! long sleep coming whether the stores are ready or not.
//!
//! * WASD or arrow keys to walk (you walk on all fours, as a bear should)
//! * Brush against berries to scoop them up; brush against a hive to steal honey
//! * Bring food to the den mouth to store it for the winter
//! * Badgers bump you, boars charge you, and bees defend their honey
//! * Clear the day's quota before dusk, then choose a boon and do it again
//! * R starts a new run, Escape quits
//!
//! The bears are pixel sprites from `resources/bears`, one 3x4 sheet per bear
//! (back, walking right, front, walking left), with the procedural [`Mesh`] rigs
//! kept as a fallback if the art is missing. The interesting parts are the
//! run/boon loop (`Boon`, `start_day`), the small steering AI in `update_foes`,
//! and the talking characters: speech bubbles are just `Speech` entries with a
//! lifetime.

use ggez::audio::{self, SoundSource};
use ggez::conf;
use ggez::event::{self, EventHandler};
use ggez::glam::*;
use ggez::graphics::{self, Color, DrawMode, DrawParam, Image, Mesh, MeshBuilder, Rect, Text};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::winit::keyboard::{Key, NamedKey, PhysicalKey};
use ggez::{Context, GameResult};
use oorandom::Rand32;
use std::time::{SystemTime, UNIX_EPOCH};

// *********************************************************************
// Tuning.
// *********************************************************************

const WINDOW_SIZE: (f32, f32) = (1024.0, 680.0);

/// Length of day one, before the days start closing in.
const DAY_ONE_SECONDS: f32 = 88.0;
/// A day can never get shorter than this, no matter how far the run goes.
const MIN_DAY_SECONDS: f32 = 48.0;
/// Food units the den needs on day one.
const FIRST_QUOTA: u32 = 6;
/// Clear this many days and the family gets its long sleep.
const WIN_DAY: u32 = 10;
/// The old badger comes to settle things on this day.
const BOSS_DAY: u32 = 5;
/// Swipe timing and reach.
const SWIPE_CD: f32 = 0.40;
const SWIPE_RANGE: f32 = 66.0;
/// Bond needed before a bear counts as family, properly.
const CLOSE_BOND: f32 = 6.0;
/// Each day asks for this much more food than the last.
const QUOTA_STEP: u32 = 2;

const CARRY_BASE: usize = 3;
const CUB_SPEED: f32 = 258.0;
const CUB_RADIUS: f32 = 20.0;
const STUN_TIME: f32 = 1.0;
const DEN_Y: f32 = 104.0;
const DEN_REACH: f32 = 100.0;
const BEE_HOME_RADIUS: f32 = 92.0;

// *********************************************************************
// Palette. Bears are warm brown, honey is gold, and dusk is orange.
// *********************************************************************

const GRASS: Color = Color::new(0.36, 0.55, 0.31, 1.0);
const GRASS_LIGHT: Color = Color::new(0.47, 0.67, 0.39, 1.0);
const GRASS_DARK: Color = Color::new(0.26, 0.43, 0.24, 1.0);
const LEAF: Color = Color::new(0.24, 0.46, 0.22, 1.0);
const CREAM: Color = Color::new(0.99, 0.96, 0.88, 1.0);
const INK: Color = Color::new(0.13, 0.10, 0.08, 1.0);
const SHINE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const SUNSET: Color = Color::new(0.98, 0.58, 0.30, 1.0);

const CUB_FUR: Color = Color::new(0.64, 0.42, 0.25, 1.0);
const CUB_FUR_DARK: Color = Color::new(0.44, 0.27, 0.15, 1.0);
const CUB_BELLY: Color = Color::new(0.87, 0.72, 0.51, 1.0);
const CUB_MUZZLE: Color = Color::new(0.94, 0.85, 0.70, 1.0);
const CUB_EAR: Color = Color::new(0.95, 0.72, 0.66, 1.0);
const CUB_BLUSH: Color = Color::new(0.93, 0.51, 0.46, 0.45);

const MAMA_FUR: Color = Color::new(0.45, 0.33, 0.24, 1.0);
const MAMA_FUR_DARK: Color = Color::new(0.30, 0.21, 0.15, 1.0);
const MAMA_BELLY: Color = Color::new(0.72, 0.62, 0.50, 1.0);
const MAMA_MUZZLE: Color = Color::new(0.86, 0.84, 0.79, 1.0);
const MAMA_EAR: Color = Color::new(0.78, 0.63, 0.57, 1.0);
const MAMA_BLUSH: Color = Color::new(0.80, 0.50, 0.45, 0.35);

const PIP_FUR: Color = Color::new(0.68, 0.50, 0.36, 1.0);
const PIP_FUR_DARK: Color = Color::new(0.48, 0.34, 0.24, 1.0);
const PIP_BELLY: Color = Color::new(0.85, 0.75, 0.61, 1.0);
const PIP_MUZZLE: Color = Color::new(0.92, 0.87, 0.78, 1.0);
const PIP_EAR: Color = Color::new(0.90, 0.72, 0.67, 1.0);
const PIP_BLUSH: Color = Color::new(0.86, 0.56, 0.50, 0.35);

const BERRY: Color = Color::new(0.79, 0.15, 0.29, 1.0);
const BERRY_LIGHT: Color = Color::new(0.96, 0.45, 0.50, 1.0);
const HONEY: Color = Color::new(0.95, 0.67, 0.15, 1.0);
const HONEY_LIGHT: Color = Color::new(1.0, 0.87, 0.47, 1.0);
const HIVE: Color = Color::new(0.72, 0.53, 0.23, 1.0);
const HIVE_DARK: Color = Color::new(0.45, 0.31, 0.13, 1.0);

const DEN_SOIL: Color = Color::new(0.31, 0.22, 0.16, 1.0);
const DEN_DARK: Color = Color::new(0.09, 0.06, 0.05, 1.0);

const BADGER_GREY: Color = Color::new(0.56, 0.55, 0.58, 1.0);
const BADGER_DARK: Color = Color::new(0.16, 0.15, 0.18, 1.0);
const BADGER_PALE: Color = Color::new(0.90, 0.89, 0.86, 1.0);

const BOAR_HIDE: Color = Color::new(0.33, 0.25, 0.21, 1.0);
const BOAR_LIGHT: Color = Color::new(0.47, 0.36, 0.30, 1.0);
const BOAR_MANE: Color = Color::new(0.20, 0.14, 0.11, 1.0);

const BEE_BODY: Color = Color::new(0.97, 0.78, 0.19, 1.0);
const BEE_STRIPE: Color = Color::new(0.18, 0.14, 0.09, 1.0);
const BEE_WING: Color = Color::new(0.90, 0.96, 1.0, 0.55);

// *********************************************************************
// Small structures.
// *********************************************************************

/// What you can carry back to the den.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Food {
    Berry,
    Honey,
    /// A ribboned honeycomb. Rare, and worth the walk back.
    Treat,
}

impl Food {
    /// How much of the winter store one of these is worth.
    fn value(self) -> u32 {
        match self {
            Food::Berry => 1,
            // Honey is rich stuff, and Pip needs it for his cough.
            Food::Honey => 2,
            Food::Treat => 3,
        }
    }
}

/// Food lying in the meadow, either spawned there or knocked out of your paws.
struct Pickup {
    pos: Vec2,
    vel: Vec2,
    kind: Food,
    wobble: f32,
}

/// A beehive. Steal from it and the bees remember you.
struct Hive {
    pos: Vec2,
    honey: u32,
    capacity: u32,
    regrow: f32,
    angry: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FoeKind {
    Badger,
    Boar,
    Bee,
}

impl Foe {
    /// How close this animal can hurt you from.
    fn reach(&self) -> f32 {
        if self.boss {
            self.kind.radius() + 16.0
        } else {
            self.kind.radius()
        }
    }
}

impl FoeKind {
    fn radius(self) -> f32 {
        match self {
            FoeKind::Badger => 24.0,
            FoeKind::Boar => 27.0,
            FoeKind::Bee => 9.0,
        }
    }
}

/// One hostile animal. All three kinds share the same little state machine:
/// calm, roused, and a short recovery after a hit.
struct Foe {
    kind: FoeKind,
    pos: Vec2,
    home: Vec2,
    /// Time left backing off after a bump, or (for bees) being angry.
    cooldown: f32,
    /// Boars pause and paw the ground before charging.
    windup: f32,
    /// How long a boar keeps charging.
    charge: f32,
    charge_dir: Vec2,
    facing: f32,
    anim: f32,
    /// Swipes left before it gives up and flees.
    hp: i32,
    /// The old badger, or something worse.
    boss: bool,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
}

/// A line of dialogue from one of the bears.
struct Speech {
    pos: Vec2,
    text: String,
    life: f32,
    max_life: f32,
    color: Color,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Title card, before anybody has carried anything.
    Title,
    /// A story panel between days.
    Cutscene,
    Playing,
    Choosing,
    RunOver,
}

/// What to do once the last story panel is dismissed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterCut {
    Resume,
    Win,
}

/// One panel of a cutscene: who is talking, and what they say.
#[derive(Clone, Copy)]
struct Cut {
    who: &'static str,
    line: &'static str,
    tint: Color,
}

// *********************************************************************
// Boons: the roguelite part. Each cleared day, pick one of three.
// *********************************************************************

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Boon {
    BiggerPaws,
    FleetPaws,
    ThickFur,
    BerryMagnet,
    HoneySense,
    BearStare,
    LongDusk,
    NoseForSnacks,
    MamasBlessing,
    SugarRush,
}

impl Boon {
    const ALL: [Boon; 10] = [
        Boon::BiggerPaws,
        Boon::FleetPaws,
        Boon::ThickFur,
        Boon::BerryMagnet,
        Boon::HoneySense,
        Boon::BearStare,
        Boon::LongDusk,
        Boon::NoseForSnacks,
        Boon::MamasBlessing,
        Boon::SugarRush,
    ];

    fn name(self) -> &'static str {
        match self {
            Boon::BiggerPaws => "Bigger Paws",
            Boon::FleetPaws => "Fleet Paws",
            Boon::ThickFur => "Thick Fur",
            Boon::BerryMagnet => "Berry Nose",
            Boon::HoneySense => "Honey Sense",
            Boon::BearStare => "Bear Stare",
            Boon::LongDusk => "Long Dusk",
            Boon::NoseForSnacks => "Snack Nose",
            Boon::MamasBlessing => "Mama's Blessing",
            Boon::SugarRush => "Sugar Rush",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Boon::BiggerPaws => "Carry one more pawful of food.",
            Boon::FleetPaws => "Walk 10% faster on all four paws.",
            Boon::ThickFur => "Shrug off a bump each day. Stacks.",
            Boon::BerryMagnet => "Scoop berries from further away.",
            Boon::HoneySense => "Hives hold more honey and regrow sooner.",
            Boon::BearStare => "Animals back off for longer after a bump.",
            Boon::LongDusk => "Eight more seconds of daylight, every day.",
            Boon::NoseForSnacks => "Berries appear more often.",
            Boon::MamasBlessing => "Start each day with food already stored.",
            Boon::SugarRush => "Honey's speed rush lasts much longer.",
        }
    }
}

// *********************************************************************
// Bear rig: the meshes that make up one bear, plus how to draw them.
// *********************************************************************

struct BearPalette {
    fur: Color,
    fur_dark: Color,
    belly: Color,
    muzzle: Color,
    ear: Color,
    blush: Color,
}

struct BearRig {
    body: Mesh,
    head: Mesh,
    head_blink: Mesh,
    leg: Mesh,
    tail: Mesh,
}

/// The pixel bears: a 3x4 sheet each, with 76x64 pixel cells.
/// Row 0 is the back view, row 1 walks right, row 2 faces you, row 3 walks left.
struct BearSprites {
    cub: Image,
    mama: Image,
    pip: Image,
}

const BEAR_CELL: Vec2 = vec2(76.0, 64.0);

/// Missing art is not fatal: the caller keeps the meshes instead.
fn load_bear_sprites(ctx: &mut Context) -> Option<BearSprites> {
    Some(BearSprites {
        cub: Image::from_path(ctx, "/bears/bear.png").ok()?,
        mama: Image::from_path(ctx, "/bears/black_bear.png").ok()?,
        pip: Image::from_path(ctx, "/bears/polar_bear.png").ok()?,
    })
}

/// Draws one pixel bear, anchored at its feet.
fn draw_bear_px(
    canvas: &mut graphics::Canvas,
    img: &Image,
    at: Vec2,
    row: usize,
    frame: usize,
    scale: f32,
    tint: Color,
) {
    // `src` is normalised UV space, not pixels, so one cell is 1/3 by 1/4.
    let src = Rect::new(
        (frame % 3) as f32 / 3.0,
        (row % 4) as f32 / 4.0,
        1.0 / 3.0,
        1.0 / 4.0,
    );
    // Pixel art: keep the edges crisp instead of smoothing them.
    canvas.set_sampler(graphics::Sampler::nearest_clamp());
    canvas.draw(
        img,
        DrawParam::new()
            .src(src)
            .dest(at - vec2(BEAR_CELL.x * scale * 0.5, BEAR_CELL.y * scale - 7.0 * scale))
            .scale(vec2(scale, scale))
            .color(tint),
    );
    canvas.set_sampler(graphics::Sampler::linear_clamp());
}

/// The player character: a very young, very round, very hopeful bear cub.
fn build_bear(ctx: &mut Context, p: &BearPalette) -> GameResult<BearRig> {
    // ---- Body: round tummy, cream chest, tiny rump.
    let mut b = MeshBuilder::new();
    b.ellipse(DrawMode::fill(), [0.0, 0.0], 22.0, 15.0, 0.1, p.fur)?;
    b.ellipse(DrawMode::fill(), [5.0, 7.0], 12.5, 7.5, 0.1, p.belly)?;
    // A soft shoulder so the head has somewhere to sit.
    b.circle(DrawMode::fill(), [10.0, -3.0], 9.0, 0.1, p.fur)?;
    let body = Mesh::from_data(ctx, b.build());

    // ---- Head: big skull, round ears, huge eyes, tiny smile.
    let head = build_head(ctx, p, false)?;
    let head_blink = build_head(ctx, p, true)?;

    // ---- One stubby leg. Origin at the hip so it can step.
    let mut l = MeshBuilder::new();
    l.ellipse(DrawMode::fill(), [0.0, 5.0], 5.2, 7.0, 0.1, p.fur)?;
    l.ellipse(DrawMode::fill(), [0.0, 10.0], 4.4, 2.8, 0.1, p.fur_dark)?;
    let leg = Mesh::from_data(ctx, l.build());

    // ---- Little round tail. Origin at its base so it can wag.
    let mut t = MeshBuilder::new();
    t.circle(DrawMode::fill(), [2.5, 0.0], 5.0, 0.1, p.fur)?;
    t.circle(DrawMode::fill(), [3.5, 0.0], 2.6, 0.1, p.ear)?;
    let tail = Mesh::from_data(ctx, t.build());

    Ok(BearRig {
        body,
        head,
        head_blink,
        leg,
        tail,
    })
}

fn build_head(ctx: &mut Context, p: &BearPalette, eyes_closed: bool) -> GameResult<Mesh> {
    let mut h = MeshBuilder::new();

    // Ears behind the skull.
    h.circle(DrawMode::fill(), [-1.0, -17.0], 6.8, 0.1, p.fur)?;
    h.circle(DrawMode::fill(), [-1.0, -17.0], 3.8, 0.1, p.ear)?;
    h.circle(DrawMode::fill(), [14.0, -19.0], 6.8, 0.1, p.fur)?;
    h.circle(DrawMode::fill(), [14.0, -19.0], 3.8, 0.1, p.ear)?;

    // Skull.
    h.circle(DrawMode::fill(), [6.0, -9.0], 12.8, 0.1, p.fur)?;

    // Muzzle and nose.
    h.ellipse(DrawMode::fill(), [14.0, -3.0], 7.8, 5.6, 0.1, p.muzzle)?;
    h.ellipse(DrawMode::fill(), [19.6, -4.6], 2.9, 2.3, 0.1, INK)?;
    // A tiny mouth line, because bears with mouths are cuter than bears without.
    h.line(&[[18.6, -0.4], [16.2, 0.2]], 1.1, INK)?;

    // Blush.
    h.ellipse(DrawMode::fill(), [-1.0, -4.0], 3.8, 2.4, 0.1, p.blush)?;
    h.ellipse(DrawMode::fill(), [17.5, -5.0], 3.2, 2.2, 0.1, p.blush)?;

    if eyes_closed {
        // Sleeping contented little arcs.
        h.line(&[[0.6, -12.4], [6.4, -12.4]], 1.7, INK)?;
        h.line(&[[11.2, -12.9], [17.0, -12.9]], 1.7, INK)?;
    } else {
        // Enormous eyes with catchlights. This is load-bearing cuteness.
        h.circle(DrawMode::fill(), [3.2, -12.2], 3.9, 0.1, INK)?;
        h.circle(DrawMode::fill(), [13.6, -12.8], 3.9, 0.1, INK)?;
        h.circle(DrawMode::fill(), [4.6, -13.6], 1.5, 0.1, SHINE)?;
        h.circle(DrawMode::fill(), [15.0, -14.2], 1.5, 0.1, SHINE)?;
    }

    Ok(Mesh::from_data(ctx, h.build()))
}

/// Draws one bear. `at` is the middle of the body, `gait` drives the leg step,
/// `facing` is +1 (right) or -1 (left), and `scale` makes mamas big and cubs small.
#[allow(clippy::too_many_arguments)]
fn draw_bear(
    canvas: &mut graphics::Canvas,
    rig: &BearRig,
    at: Vec2,
    facing: f32,
    scale: f32,
    gait: f32,
    head_tilt: f32,
    blink: bool,
    tint: Color,
) {
    let bob = (gait * 2.0).sin() * 1.5;
    let body_at = at + vec2(0.0, bob) * scale;

    // Far legs, drawn darker and behind the body.
    for (i, leg_x) in [-13.0_f32, 9.0].iter().enumerate() {
        let step = (gait + std::f32::consts::PI * i as f32).sin() * 2.6;
        canvas.draw(
            &rig.leg,
            DrawParam::new()
                .dest(body_at + vec2(facing * leg_x * scale, (12.5 + step) * scale))
                .scale(vec2(facing * scale, scale))
                .color(dim(tint, 0.76)),
        );
    }

    // Tail, wagging gently.
    canvas.draw(
        &rig.tail,
        DrawParam::new()
            .dest(body_at + vec2(-facing * 20.0 * scale, -3.0 * scale))
            .scale(vec2(facing * scale, scale))
            .rotation((gait * 1.4).sin() * 0.25)
            .color(tint),
    );

    canvas.draw(
        &rig.body,
        DrawParam::new()
            .dest(body_at)
            .scale(vec2(facing * scale, scale))
            .color(tint),
    );

    // Near legs on top of the body.
    for (i, leg_x) in [-9.0_f32, 13.0].iter().enumerate() {
        let step = (gait + std::f32::consts::PI * (i as f32 + 1.0)).sin() * 2.6;
        canvas.draw(
            &rig.leg,
            DrawParam::new()
                .dest(body_at + vec2(facing * leg_x * scale, (13.0 + step) * scale))
                .scale(vec2(facing * scale, scale))
                .color(tint),
        );
    }

    canvas.draw(
        if blink { &rig.head_blink } else { &rig.head },
        DrawParam::new()
            .dest(body_at + vec2(facing * 13.0 * scale, -9.0 * scale))
            .scale(vec2(facing * scale * 1.08, scale * 1.08))
            .rotation(head_tilt)
            .color(tint),
    );
}

// *********************************************************************
// The other animals, and the props.
// *********************************************************************

fn build_badger(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    // Legs first so the body covers their tops.
    for x in [-16.0_f32, -6.0, 8.0, 18.0] {
        b.ellipse(DrawMode::fill(), [x, 11.0], 4.6, 6.0, 0.1, BADGER_DARK)?;
    }
    // Body.
    b.ellipse(DrawMode::fill(), [0.0, 0.0], 25.0, 14.0, 0.1, BADGER_GREY)?;
    b.ellipse(DrawMode::fill(), [2.0, 5.0], 17.0, 8.0, 0.1, BADGER_PALE)?;
    b.circle(DrawMode::fill(), [-24.0, -4.0], 4.5, 0.1, BADGER_PALE)?;
    // Head, low and wide, with the classic stripes.
    b.circle(DrawMode::fill(), [23.0, -5.0], 11.5, 0.1, BADGER_GREY)?;
    b.ellipse(DrawMode::fill(), [33.0, -2.0], 5.5, 4.2, 0.1, BADGER_PALE)?;
    b.circle(DrawMode::fill(), [37.0, -3.0], 2.2, 0.1, BADGER_DARK)?;
    b.ellipse(DrawMode::fill(), [19.0, -8.0], 3.2, 7.5, 0.1, BADGER_DARK)?;
    b.ellipse(DrawMode::fill(), [28.0, -8.5], 3.0, 7.0, 0.1, BADGER_DARK)?;
    b.ellipse(DrawMode::fill(), [24.0, -9.5], 1.9, 6.0, 0.1, BADGER_PALE)?;
    b.circle(DrawMode::fill(), [16.0, -17.0], 3.6, 0.1, BADGER_DARK)?;
    b.circle(DrawMode::fill(), [26.0, -17.5], 3.6, 0.1, BADGER_DARK)?;
    // Eyes.
    b.circle(DrawMode::fill(), [20.0, -6.0], 1.7, 0.1, INK)?;
    b.circle(DrawMode::fill(), [28.0, -6.4], 1.7, 0.1, INK)?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_boar(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    for x in [-14.0_f32, -4.0, 10.0, 20.0] {
        b.ellipse(DrawMode::fill(), [x, 12.0], 4.2, 5.4, 0.1, BOAR_MANE)?;
    }
    // Heavy body.
    b.ellipse(DrawMode::fill(), [0.0, -1.0], 26.0, 16.0, 0.1, BOAR_HIDE)?;
    b.ellipse(DrawMode::fill(), [2.0, 5.0], 18.0, 8.0, 0.1, BOAR_LIGHT)?;
    // Bristly mane along the spine.
    for i in 0..6 {
        let x = -14.0 + i as f32 * 5.2;
        let tip = -20.0 - (i as f32 * 1.4).sin() * 3.0;
        b.polygon(
            DrawMode::fill(),
            &[[x - 2.6, -13.0], [x, tip], [x + 2.6, -13.0]],
            BOAR_MANE,
        )?;
    }
    // Head with snout and tusks.
    b.circle(DrawMode::fill(), [23.0, 0.0], 12.0, 0.1, BOAR_HIDE)?;
    b.ellipse(DrawMode::fill(), [34.0, 3.0], 6.0, 4.8, 0.1, BOAR_LIGHT)?;
    b.circle(DrawMode::fill(), [37.0, 2.0], 1.8, 0.1, BOAR_MANE)?;
    b.ellipse(DrawMode::fill(), [25.0, -12.0], 3.2, 3.0, 0.1, BOAR_MANE)?;
    // Tusks.
    b.line(&[[31.0, 6.0], [34.0, 1.0]], 2.6, CREAM)?;
    b.line(&[[28.0, 7.0], [30.5, 2.0]], 2.0, CREAM)?;
    b.circle(DrawMode::fill(), [22.0, -2.0], 1.8, 0.1, INK)?;
    b.circle(DrawMode::fill(), [29.0, -2.4], 1.7, 0.1, INK)?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_bee(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    // Wings.
    b.ellipse(DrawMode::fill(), [-2.0, -6.0], 6.0, 3.6, 0.2, BEE_WING)?;
    b.ellipse(DrawMode::fill(), [3.0, -6.6], 5.2, 3.2, 0.2, BEE_WING)?;
    // Body.
    b.ellipse(DrawMode::fill(), [0.0, 0.0], 7.0, 5.0, 0.1, BEE_BODY)?;
    b.ellipse(DrawMode::fill(), [-1.0, 0.0], 1.6, 4.8, 0.1, BEE_STRIPE)?;
    b.ellipse(DrawMode::fill(), [3.4, 0.0], 1.5, 4.4, 0.1, BEE_STRIPE)?;
    // Head and stinger.
    b.circle(DrawMode::fill(), [-7.0, -0.5], 3.4, 0.1, BEE_STRIPE)?;
    b.polygon(
        DrawMode::fill(),
        &[[7.0, -1.2], [10.5, 0.0], [7.0, 1.2]],
        BEE_STRIPE,
    )?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_hive(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    b.ellipse(
        DrawMode::fill(),
        [0.0, 16.0],
        15.0,
        5.0,
        0.2,
        Color::new(0.0, 0.0, 0.0, 0.18),
    )?;
    b.ellipse(DrawMode::fill(), [0.0, 2.0], 16.0, 12.0, 0.1, HIVE)?;
    b.ellipse(DrawMode::fill(), [0.0, -8.0], 12.0, 9.0, 0.1, HIVE)?;
    b.ellipse(DrawMode::fill(), [0.0, -16.0], 7.5, 6.0, 0.1, HIVE)?;
    b.ellipse(DrawMode::fill(), [0.0, 8.0], 5.0, 3.0, 0.1, HIVE_DARK)?;
    b.ellipse(DrawMode::fill(), [-5.0, -2.0], 4.0, 6.0, 0.1, HONEY_LIGHT)?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_berry(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    b.circle(DrawMode::fill(), [0.0, 0.0], 8.0, 0.1, BERRY)?;
    b.circle(DrawMode::fill(), [-2.4, -2.4], 2.6, 0.1, BERRY_LIGHT)?;
    b.ellipse(DrawMode::fill(), [3.6, -8.0], 4.2, 2.2, 0.1, LEAF)?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_honey(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    b.circle(DrawMode::fill(), [0.0, 0.0], 7.0, 0.1, HONEY)?;
    b.ellipse(DrawMode::fill(), [0.0, 6.0], 2.6, 3.4, 0.1, HONEY)?;
    b.circle(DrawMode::fill(), [-2.0, -2.4], 2.2, 0.1, HONEY_LIGHT)?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_treat(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    // Honeycomb, wrapped in a leaf and tied with a berry-red ribbon.
    b.ellipse(DrawMode::fill(), [0.0, 0.0], 8.5, 7.0, 0.1, HONEY)?;
    b.ellipse(DrawMode::fill(), [0.0, 1.5], 6.0, 4.2, 0.1, HONEY_LIGHT)?;
    b.line(&[[-8.0, -4.0], [8.0, -4.0]], 1.6, LEAF)?;
    b.line(&[[0.0, -9.0], [0.0, 8.0]], 1.4, BERRY)?;
    b.polygon(
        DrawMode::fill(),
        &[[0.0, -9.5], [4.5, -6.0], [0.0, -2.5], [-4.5, -6.0]],
        BERRY,
    )?;
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_den(ctx: &mut Context) -> GameResult<Mesh> {
    let mut b = MeshBuilder::new();
    b.ellipse(DrawMode::fill(), [0.0, 18.0], 150.0, 64.0, 0.1, DEN_SOIL)?;
    b.ellipse(
        DrawMode::fill(),
        [0.0, 24.0],
        104.0,
        42.0,
        0.1,
        CUB_FUR_DARK,
    )?;
    b.ellipse(DrawMode::fill(), [0.0, 28.0], 58.0, 30.0, 0.1, DEN_DARK)?;
    // Two rocks framing the door.
    b.ellipse(
        DrawMode::fill(),
        [-108.0, 32.0],
        17.0,
        11.0,
        0.1,
        GRASS_DARK,
    )?;
    b.ellipse(DrawMode::fill(), [104.0, 36.0], 14.0, 9.0, 0.1, GRASS_DARK)?;
    // A few tufts on the mound.
    for x in [-70.0_f32, -40.0, 60.0, 86.0] {
        b.line(&[[x, 6.0], [x + 3.0, -7.0]], 1.6, LEAF)?;
    }
    Ok(Mesh::from_data(ctx, b.build()))
}

fn build_decor(ctx: &mut Context) -> GameResult<Mesh> {
    let mut rng = seeded_rng();
    let (w, h) = WINDOW_SIZE;
    let mut b = MeshBuilder::new();

    for _ in 0..8 {
        let pos = [
            50.0 + rng.rand_float() * (w - 100.0),
            130.0 + rng.rand_float() * (h - 190.0),
        ];
        b.ellipse(
            DrawMode::fill(),
            pos,
            70.0 + rng.rand_float() * 100.0,
            34.0 + rng.rand_float() * 44.0,
            0.6,
            Color::new(GRASS_LIGHT.r, GRASS_LIGHT.g, GRASS_LIGHT.b, 0.30),
        )?;
    }

    for _ in 0..80 {
        let x = 20.0 + rng.rand_float() * (w - 40.0);
        let y = 140.0 + rng.rand_float() * (h - 175.0);
        for blade in 0..3 {
            let lean = (blade as f32 - 1.0) * 3.0;
            b.line(
                &[
                    [x + lean, y],
                    [x + lean * 1.9, y - 7.0 - rng.rand_float() * 5.0],
                ],
                1.5,
                LEAF,
            )?;
        }
    }

    for _ in 0..18 {
        let x = 40.0 + rng.rand_float() * (w - 80.0);
        let y = 150.0 + rng.rand_float() * (h - 180.0);
        let petal = if rng.rand_float() > 0.5 {
            Color::new(0.98, 0.92, 0.60, 0.95)
        } else {
            Color::new(0.96, 0.75, 0.83, 0.95)
        };
        for i in 0..5 {
            let angle = i as f32 / 5.0 * std::f32::consts::TAU;
            b.circle(
                DrawMode::fill(),
                [x + angle.cos() * 2.6, y + angle.sin() * 2.6],
                2.1,
                0.15,
                petal,
            )?;
        }
        b.circle(DrawMode::fill(), [x, y], 1.4, 0.15, SUNSET)?;
    }

    Ok(Mesh::from_data(ctx, b.build()))
}

// *********************************************************************
// Sound: one shot effects, queued and played on the next frame.
// *********************************************************************

const SFX_PICKUP: usize = 0;
const SFX_DELIVER: usize = 1;
const SFX_HONEY: usize = 2;
const SFX_BUMP: usize = 3;
const SFX_FUR: usize = 4;
const SFX_DAY_CLEAR: usize = 5;
const SFX_CHOOSE: usize = 6;
const SFX_OVER: usize = 7;
const SFX_WIN: usize = 8;

/// Every effect is optional: a machine with no audio device just plays in
/// silence instead of failing to start. Sounds are queued rather than played
/// on the spot so that overlapping one-shots each get their own detached sink.
struct Sfx {
    data: Vec<Option<audio::SoundData>>,
    pending: Vec<usize>,
    muted: bool,
}

impl Sfx {
    fn load(ctx: &mut Context) -> Self {
        const FILES: [&str; 9] = [
            "/sounds/pickup.wav",
            "/sounds/deliver.wav",
            "/sounds/honey.wav",
            "/sounds/bump.wav",
            "/sounds/nope.wav",
            "/sounds/day_clear.wav",
            "/sounds/choose.wav",
            "/sounds/game_over.wav",
            "/sounds/win.wav",
        ];
        let data = FILES
            .iter()
            .map(|path| audio::SoundData::new(ctx, path).ok())
            .collect();
        Sfx {
            data,
            pending: Vec::new(),
            muted: false,
        }
    }

    fn request(&mut self, id: usize) {
        if !self.muted {
            self.pending.push(id);
        }
    }

    fn flush(&mut self, ctx: &Context) {
        if self.muted {
            self.pending.clear();
            return;
        }
        for id in std::mem::take(&mut self.pending) {
            if let Some(data) = self.data.get(id).and_then(|d| d.as_ref()) {
                if let Ok(mut source) = audio::Source::from_data(ctx, data.clone()) {
                    source.set_volume(0.55);
                    source.play_detached();
                }
            }
        }
    }
}

/// Small talk while the day burns down.
const CHATTER: [&str; 12] = [
    "Your father filled this den in four days. Four! I counted.",
    "Do not eat the ones with the pale spots. Those are for the birds.",
    "You used to fit in the crook of my paw. Now look at you. Walking.",
    "If the badger comes at you sideways, that is a bluff. Mostly.",
    "Snow first, then sleep. Snow first, then sleep. That is the whole year.",
    "Pip coughed less last night. I did not sleep anyway.",
    "Honey is not food, it is medicine. That is why it tastes like that.",
    "When I am asleep, do not wake me. Unless it is important. Or honey.",
    "You are doing better than you think, little one.",
    "The ridge is where the berries get fat. Too far for me now.",
    "Come back before dusk. I will say it every day until I am gone.",
    "You smell like the meadow. Good. Bring that in with you.",
];

// *********************************************************************
// The game.
// *********************************************************************

struct Game {
    rng: Rand32,
    size: Vec2,
    clock: f32,

    // Models, built once.
    cub_rig: BearRig,
    mama_rig: BearRig,
    pip_rig: BearRig,
    /// Pixel bears from `resources/bears`; `None` falls back to the rigs above.
    sprites: Option<BearSprites>,
    badger_mesh: Mesh,
    boar_mesh: Mesh,
    bee_mesh: Mesh,
    hive_mesh: Mesh,
    berry_mesh: Mesh,
    honey_mesh: Mesh,
    den_mesh: Mesh,
    decor_mesh: Mesh,
    dot_mesh: Mesh,

    // Run state.
    phase: Phase,
    day: u32,
    quota: u32,
    stored: u32,
    day_time: f32,
    day_len: f32,
    total_food: u32,
    best_day: u32,
    runs: u32,
    over_reason: String,
    boons: Vec<Boon>,
    offers: Vec<Boon>,

    // Player.
    cub: Vec2,
    cub_vel: Vec2,
    facing: f32,
    gait: f32,
    /// Drives the walk cycle, and which row of the sheet is showing.
    anim_time: f32,
    pose_row: usize,
    stun: f32,
    carried: Vec<Food>,
    rush: f32,
    blink_timer: f32,
    blinking: f32,
    fur_left: u32,

    // World.
    pickups: Vec<Pickup>,
    hives: Vec<Hive>,
    foes: Vec<Foe>,
    particles: Vec<Particle>,
    speech: Vec<Speech>,
    spawn_timer: f32,
    quip_cd: f32,
    /// Pause between honey grabs, so hugging a hive does not empty it.
    grab_cd: f32,
    mama_hop: f32,
    pip_wiggle: f32,
    hurt_flash: f32,

    /// Queued sound effects, flushed once a frame.
    sfx: Sfx,
    /// True when the run finished in the long sleep instead of at dusk.
    won: bool,

    // Story, family and comfort.
    cut: Vec<Cut>,
    cut_i: usize,
    cut_after: AfterCut,
    /// How loved the family feels. Feeds the little perks.
    bond_mama: f32,
    bond_pip: f32,
    /// Bruises carried from day to day. Slows a cub down.
    injury: u32,
    cuddle_cd: f32,
    cuddle_streak: u32,
    chat_cd: f32,
    chat_i: usize,
    seen_mid: bool,
    boss_down: bool,
    swipe_cd: f32,
    swipe_anim: f32,
    toast: Option<(String, f32)>,
    debug: bool,
    cheat_buf: String,
    adam: bool,
    title_page: u8,
    treat_mesh: Mesh,
}

impl Game {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        ctx.gfx.add_font(
            "LiberationMono",
            graphics::FontData::from_path(ctx, "/LiberationMono-Regular.ttf")?,
        );

        let cub_rig = build_bear(
            ctx,
            &BearPalette {
                fur: CUB_FUR,
                fur_dark: CUB_FUR_DARK,
                belly: CUB_BELLY,
                muzzle: CUB_MUZZLE,
                ear: CUB_EAR,
                blush: CUB_BLUSH,
            },
        )?;
        let mama_rig = build_bear(
            ctx,
            &BearPalette {
                fur: MAMA_FUR,
                fur_dark: MAMA_FUR_DARK,
                belly: MAMA_BELLY,
                muzzle: MAMA_MUZZLE,
                ear: MAMA_EAR,
                blush: MAMA_BLUSH,
            },
        )?;
        let pip_rig = build_bear(
            ctx,
            &BearPalette {
                fur: PIP_FUR,
                fur_dark: PIP_FUR_DARK,
                belly: PIP_BELLY,
                muzzle: PIP_MUZZLE,
                ear: PIP_EAR,
                blush: PIP_BLUSH,
            },
        )?;

        let sprites = load_bear_sprites(ctx);

        let mut game = Game {
            rng: seeded_rng(),
            size: vec2(WINDOW_SIZE.0, WINDOW_SIZE.1),
            clock: 0.0,
            cub_rig,
            mama_rig,
            pip_rig,
            sprites,
            badger_mesh: build_badger(ctx)?,
            boar_mesh: build_boar(ctx)?,
            bee_mesh: build_bee(ctx)?,
            hive_mesh: build_hive(ctx)?,
            berry_mesh: build_berry(ctx)?,
            honey_mesh: build_honey(ctx)?,
            den_mesh: build_den(ctx)?,
            decor_mesh: build_decor(ctx)?,
            dot_mesh: Mesh::new_circle(ctx, DrawMode::fill(), [0.0, 0.0], 1.0, 0.1, Color::WHITE)?,
            phase: Phase::Playing,
            day: 1,
            quota: FIRST_QUOTA,
            stored: 0,
            day_time: DAY_ONE_SECONDS,
            day_len: DAY_ONE_SECONDS,
            total_food: 0,
            best_day: 0,
            runs: 0,
            over_reason: String::new(),
            boons: Vec::new(),
            offers: Vec::new(),
            cub: Vec2::ZERO,
            cub_vel: Vec2::ZERO,
            facing: 1.0,
            gait: 0.0,
            anim_time: 0.0,
            pose_row: 1,
            stun: 0.0,
            carried: Vec::new(),
            rush: 0.0,
            blink_timer: 3.0,
            blinking: 0.0,
            fur_left: 0,
            pickups: Vec::new(),
            hives: Vec::new(),
            foes: Vec::new(),
            particles: Vec::new(),
            speech: Vec::new(),
            spawn_timer: 0.0,
            quip_cd: 0.0,
            grab_cd: 0.0,
            mama_hop: 0.0,
            pip_wiggle: 0.0,
            hurt_flash: 0.0,
            sfx: Sfx::load(ctx),
            won: false,
            cut: Vec::new(),
            cut_i: 0,
            cut_after: AfterCut::Resume,
            bond_mama: 0.0,
            bond_pip: 0.0,
            injury: 0,
            cuddle_cd: 0.0,
            cuddle_streak: 0,
            chat_cd: 9.0,
            chat_i: 0,
            seen_mid: false,
            boss_down: false,
            swipe_cd: 0.0,
            swipe_anim: 0.0,
            toast: None,
            debug: false,
            cheat_buf: String::new(),
            adam: false,
            title_page: 0,
            treat_mesh: build_treat(ctx)?,
        };
        game.start_run();
        game.phase = Phase::Title;
        Ok(game)
    }

    // ---- Run and day lifecycle ------------------------------------

    fn start_run(&mut self) {
        self.runs += 1;
        self.day = 1;
        self.total_food = 0;
        self.boons.clear();
        self.offers.clear();
        self.bond_mama = 0.0;
        self.bond_pip = 0.0;
        self.injury = 0;
        self.boss_down = false;
        self.seen_mid = false;
        self.adam = false;
        self.cuddle_streak = 0;
        self.cut.clear();
        self.start_day();
    }

    fn start_day(&mut self) {
        self.day_len = (DAY_ONE_SECONDS - (self.day - 1) as f32 * 3.0).max(MIN_DAY_SECONDS)
            + 8.0 * self.count(Boon::LongDusk) as f32;
        self.day_time = self.day_len;
        self.quota = FIRST_QUOTA + (self.day - 1) * QUOTA_STEP;
        self.stored = self.count(Boon::MamasBlessing) as u32;
        self.phase = Phase::Playing;
        self.won = false;

        self.cub = vec2(self.size.x * 0.5, DEN_Y + 168.0);
        self.cub_vel = Vec2::ZERO;
        self.facing = 1.0;
        self.gait = 0.0;
        self.stun = 0.0;
        self.carried.clear();
        self.rush = 0.0;
        self.fur_left = self.count(Boon::ThickFur) as u32;
        self.pickups.clear();
        self.hives.clear();
        self.foes.clear();
        self.particles.clear();
        self.speech.clear();
        self.spawn_timer = 0.0;
        self.mama_hop = 0.0;
        self.pip_wiggle = 0.0;
        self.hurt_flash = 0.0;

        self.build_meadow();

        let aside = self.day_intro();
        self.say(self.den_center() + vec2(-104.0, -78.0), aside, CREAM, 4.5);
    }

    /// Escalation: every day adds a little more trouble.
    fn build_meadow(&mut self) {
        // Berries already waiting in the grass.
        for _ in 0..5 {
            self.spawn_berry();
        }

        // Hives from day three, guarded by bees.
        let hive_count = if self.day >= 5 {
            2
        } else if self.day >= 3 {
            1
        } else {
            0
        };
        for i in 0..hive_count {
            let pos = vec2(
                self.size.x * (0.22 + i as f32 * 0.56),
                self.size.y * 0.45 + i as f32 * 60.0,
            );
            let capacity = 2 + self.count(Boon::HoneySense) as u32;
            self.hives.push(Hive {
                pos,
                honey: capacity,
                capacity,
                regrow: 0.0,
                angry: 0.0,
            });
            for b in 0..2 {
                self.foes.push(Foe {
                    kind: FoeKind::Bee,
                    hp: 1,
                    boss: false,
                    pos: pos + vec2(20.0 * b as f32, -14.0),
                    home: pos,
                    cooldown: 0.0,
                    windup: 0.0,
                    charge: 0.0,
                    charge_dir: Vec2::ZERO,
                    facing: 1.0,
                    anim: self.rng.rand_float() * 6.0,
                });
            }
        }

        // Badgers: one, then two from day four.
        let badger_count = if self.day >= 4 { 2 } else { 1 };
        for i in 0..badger_count {
            let home = vec2(
                if i == 0 { self.size.x - 140.0 } else { 140.0 },
                self.size.y - 130.0,
            );
            self.foes.push(Foe {
                kind: FoeKind::Badger,
                hp: 2,
                boss: false,
                pos: home,
                home,
                cooldown: 0.0,
                windup: 0.0,
                charge: 0.0,
                charge_dir: Vec2::ZERO,
                facing: -1.0,
                anim: self.rng.rand_float() * 6.0,
            });
        }

        // The old badger stops playing games once the stores grow fat.
        if self.day >= BOSS_DAY && !self.boss_down {
            if let Some(b) = self.foes.iter_mut().find(|f| f.kind == FoeKind::Badger) {
                b.boss = true;
                b.hp = 8;
            }
        }

        // A treat waiting in the grass, because somebody always drops one.
        if self.day >= 3 {
            let spot = vec2(
                self.size.x * (0.3 + self.rng.rand_float() * 0.4),
                self.size.y * (0.55 + self.rng.rand_float() * 0.3),
            );
            self.pickups.push(Pickup {
                pos: spot,
                vel: Vec2::ZERO,
                kind: Food::Treat,
                wobble: 0.0,
            });
        }

        // Boars from day two, a second one from day five.
        let boar_count = if self.day >= 5 {
            2
        } else if self.day >= 2 {
            1
        } else {
            0
        };
        for i in 0..boar_count {
            let home = vec2(
                self.size.x * (0.25 + 0.5 * i as f32),
                self.size.y * 0.62 + 40.0 * i as f32,
            );
            self.foes.push(Foe {
                kind: FoeKind::Boar,
                hp: 3,
                boss: false,
                pos: home,
                home,
                cooldown: 0.0,
                windup: 0.0,
                charge: 0.0,
                charge_dir: Vec2::ZERO,
                facing: -1.0,
                anim: self.rng.rand_float() * 6.0,
            });
        }
    }

    fn day_intro(&self) -> String {
        match self.day {
            1 => "Little one, the den is empty. Bring me berries before dusk.".into(),
            2 => "The old badger is still cross with you. Watch his shoulders.".into(),
            3 => "Honey for Pip would warm his cough. Mind the bees, mind them!".into(),
            4 => "There are two badgers now. Do not try to out-stubborn them.".into(),
            _ => format!("Day {}. The cold is at the door now, little one.", self.day),
        }
    }

    fn day_clear_line(&self) -> String {
        match self.day {
            1 => "Good cub. Now go and choose what kind of bear you'll be.".into(),
            2 => "Pip sniffed the air and smiled. That is thanks enough.".into(),
            _ => "The den grows heavy. Our long sleep will be a soft one.".into(),
        }
    }

    // ---- Boons ----------------------------------------------------

    fn count(&self, boon: Boon) -> usize {
        self.boons.iter().filter(|b| **b == boon).count()
    }

    fn carry_limit(&self) -> usize {
        let loved = if self.bond_mama >= CLOSE_BOND { 1 } else { 0 };
        CARRY_BASE + self.count(Boon::BiggerPaws) + loved
    }

    fn speed(&self) -> f32 {
        let base = CUB_SPEED * (1.0 + 0.10 * self.count(Boon::FleetPaws) as f32);
        let rushed = if self.rush > 0.0 { 1.28 } else { 1.0 };
        // Bruises slow a cub down; being loved makes up for some of it.
        let bruised = 1.0 - 0.07 * self.injury as f32;
        let comfort = 1.0 + 0.02 * (self.bond_mama + self.bond_pip);
        base * rushed * bruised * comfort
    }

    fn pickup_reach(&self) -> f32 {
        CUB_RADIUS + 9.0 + 14.0 * self.count(Boon::BerryMagnet) as f32
    }

    fn offer_boons(&mut self) {
        let mut pool: Vec<Boon> = Boon::ALL.to_vec();
        self.offers.clear();
        for _ in 0..3 {
            if pool.is_empty() {
                break;
            }
            let i = self.rng.rand_range(0..pool.len() as u32) as usize;
            self.offers.push(pool.remove(i));
        }
    }

    fn take_boon(&mut self, index: usize) {
        if index >= self.offers.len() {
            return;
        }
        let boon = self.offers[index];
        self.boons.push(boon);
        self.sfx.request(SFX_CHOOSE);
        self.offers.clear();
        self.day += 1;
        self.start_day();
        // The old badger gets a proper introduction.
        if self.day == BOSS_DAY && !self.seen_mid {
            self.seen_mid = true;
            self.begin_cut(Self::interlude_panels(), AfterCut::Resume);
        }
    }

    // ---- Speech and sparkles --------------------------------------

    fn say(&mut self, pos: Vec2, text: impl Into<String>, color: Color, seconds: f32) {
        self.speech.push(Speech {
            pos,
            text: text.into(),
            life: seconds,
            max_life: seconds,
            color,
        });
    }

    /// Cub comments are rate limited so the game doesn't turn into a chat app.
    fn quip(&mut self, text: impl Into<String>) {
        if self.quip_cd > 0.0 {
            return;
        }
        self.quip_cd = 2.6;
        let at = self.cub + vec2(0.0, -58.0);
        self.say(at, text, CREAM, 1.5);
    }

    fn sparkle(&mut self, at: Vec2, color: Color, count: usize) {
        for i in 0..count {
            let angle =
                (i as f32 / count.max(1) as f32) * std::f32::consts::TAU + self.rng.rand_float();
            let speed = 40.0 + self.rng.rand_float() * 95.0;
            let life = 0.35 + self.rng.rand_float() * 0.4;
            self.particles.push(Particle {
                pos: at,
                vel: vec2(angle.cos(), angle.sin()) * speed,
                life,
                max_life: life,
                size: 2.0 + self.rng.rand_float() * 3.0,
                color,
            });
        }
    }

    // ---- World helpers --------------------------------------------

    fn den_center(&self) -> Vec2 {
        vec2(self.size.x * 0.5, DEN_Y)
    }

    fn spawn_berry(&mut self) {
        let margin = 70.0;
        let den = self.den_center();
        for _ in 0..28 {
            let pos = vec2(
                margin + self.rng.rand_float() * (self.size.x - margin * 2.0),
                DEN_Y + 150.0 + self.rng.rand_float() * (self.size.y - DEN_Y - 150.0 - margin),
            );
            if pos.distance(den) > 160.0 && pos.distance(self.cub) > 70.0 {
                self.pickups.push(Pickup {
                    pos,
                    vel: Vec2::ZERO,
                    kind: Food::Berry,
                    wobble: self.rng.rand_float() * 6.0,
                });
                return;
            }
        }
    }

    // ---- Update ---------------------------------------------------

    fn update(&mut self, ctx: &mut Context) {
        let dt = ctx.time.delta().as_secs_f32().min(1.0 / 20.0);
        let (w, h) = ctx.gfx.size();
        self.size = vec2(w, h);
        self.clock += dt;

        match self.phase {
            Phase::Playing => {
                let dir = self.read_input(ctx);
                self.update_day(dt, dir);
            }
            Phase::Title | Phase::Cutscene | Phase::Choosing | Phase::RunOver => {}
        }

        self.update_effects(dt);
        self.sfx.flush(ctx);
        ggez::timer::yield_now();
    }

    fn read_input(&self, ctx: &Context) -> Vec2 {
        let mut dir = Vec2::ZERO;
        if held(ctx, KeyCode::KeyA) || held(ctx, KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if held(ctx, KeyCode::KeyD) || held(ctx, KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if held(ctx, KeyCode::KeyW) || held(ctx, KeyCode::ArrowUp) {
            dir.y -= 1.0;
        }
        if held(ctx, KeyCode::KeyS) || held(ctx, KeyCode::ArrowDown) {
            dir.y += 1.0;
        }
        dir.normalize_or_zero()
    }

    fn update_day(&mut self, dt: f32, dir: Vec2) {
        self.day_time = (self.day_time - dt).max(0.0);
        self.rush = (self.rush - dt).max(0.0);
        self.swipe_cd = (self.swipe_cd - dt).max(0.0);
        self.swipe_anim = (self.swipe_anim - dt).max(0.0);
        self.cuddle_cd = (self.cuddle_cd - dt).max(0.0);
        self.ambient_chat(dt);

        // ---- The cub ----
        if self.stun > 0.0 {
            self.stun = (self.stun - dt).max(0.0);
            self.cub_vel *= 0.82;
        } else if dir.length_squared() > 0.0 {
            let target = dir * self.speed();
            self.cub_vel += (target - self.cub_vel) * (dt * 14.0).min(1.0);
            if dir.x.abs() > 0.05 {
                self.facing = dir.x.signum();
            }
            // Which row of the bear sheet to show: away, right, toward, left.
            if dir.y.abs() > dir.x.abs() {
                self.pose_row = if dir.y < 0.0 { 0 } else { 2 };
            } else if dir.x.abs() > 0.05 {
                self.pose_row = if dir.x > 0.0 { 1 } else { 3 };
            }
            self.anim_time += dt;
            self.gait += dt * 10.0;
        } else {
            self.cub_vel *= (1.0 - dt * 12.0).clamp(0.0, 1.0);
            self.gait += dt * 1.6;
        }

        self.cub += self.cub_vel * dt;
        let r = CUB_RADIUS;
        self.cub.x = self.cub.x.clamp(r, self.size.x - r);
        self.cub.y = self.cub.y.clamp(92.0, self.size.y - r);

        // Blinking, because a cub that never blinks is unsettling.
        self.blink_timer -= dt;
        if self.blink_timer <= 0.0 {
            self.blinking = 0.11;
            self.blink_timer = 2.4 + self.rng.rand_float() * 3.0;
        }
        self.blinking = (self.blinking - dt).max(0.0);

        // ---- Hives ----
        for i in 0..self.hives.len() {
            let hive_pos = self.hives[i].pos;
            self.hives[i].angry = (self.hives[i].angry - dt).max(0.0);

            // Regrow honey slowly.
            if self.hives[i].honey < self.hives[i].capacity {
                self.hives[i].regrow += dt;
                let regrow_time = if self.count(Boon::HoneySense) > 0 {
                    5.0
                } else {
                    8.0
                };
                if self.hives[i].regrow >= regrow_time {
                    self.hives[i].regrow = 0.0;
                    self.hives[i].honey += 1;
                }
            }

            // Stealing honey wakes the swarm.
            if self.hives[i].honey > 0
                && self.carried.len() < self.carry_limit()
                && self.grab_cd <= 0.0
                && self.cub.distance(hive_pos) < 44.0
            {
                self.hives[i].honey -= 1;
                self.hives[i].angry = 4.0;
                self.grab_cd = 0.55;
                self.carried.push(Food::Honey);
                self.sfx.request(SFX_HONEY);
                self.rush = 3.0 + 1.6 * self.count(Boon::SugarRush) as f32;
                self.sparkle(hive_pos, HONEY_LIGHT, 8);
                self.quip("HONEY!");
                for foe in &mut self.foes {
                    if foe.kind == FoeKind::Bee && foe.home.distance(hive_pos) < 1.0 {
                        foe.cooldown = 4.0;
                    }
                }
            }
        }

        // ---- Pickups ----
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            let cap = 7 + self.count(Boon::NoseForSnacks) * 2;
            if self.pickups.len() < cap {
                self.spawn_berry();
            }
            let gap = if self.count(Boon::NoseForSnacks) > 0 {
                0.45
            } else {
                0.75
            };
            self.spawn_timer = gap;
        }

        for pickup in &mut self.pickups {
            pickup.pos += pickup.vel * dt;
            pickup.vel *= (1.0 - dt * 6.0).clamp(0.0, 1.0);
            pickup.pos.x = pickup.pos.x.clamp(10.0, self.size.x - 10.0);
            pickup.pos.y = pickup.pos.y.clamp(10.0, self.size.y - 10.0);
            pickup.wobble += dt * 3.0;
        }

        if self.carried.len() < self.carry_limit() {
            let reach = self.pickup_reach();
            let mut picked: Vec<usize> = Vec::new();
            for (i, pickup) in self.pickups.iter().enumerate() {
                if pickup.pos.distance(self.cub) < reach
                    && self.carried.len() + picked.len() < self.carry_limit()
                {
                    picked.push(i);
                }
            }
            let picked_count = picked.len();
            for i in picked.into_iter().rev() {
                let pickup = self.pickups.remove(i);
                self.carried.push(pickup.kind);
                self.sparkle(pickup.pos, pickup.kind_color(), 4);
                if pickup.kind == Food::Berry && self.rng.rand_float() > 0.7 {
                    self.quip("Mine!");
                }
            }
            if picked_count > 0 {
                self.sfx.request(SFX_PICKUP);
            }
        }

        // ---- Deliveries ----
        let den = self.den_center();
        if !self.carried.is_empty() && self.cub.distance(den) < DEN_REACH {
            let mut honey_in_load = 0;
            for food in self.carried.drain(..) {
                if food == Food::Honey {
                    honey_in_load += 1;
                }
                self.stored += food.value();
                self.total_food += food.value();
            }
            self.sfx.request(SFX_DELIVER);
            self.mama_hop = 0.6;
            self.pip_wiggle = 0.9;
            self.sparkle(den + vec2(0.0, 12.0), HONEY_LIGHT, 10);
            if honey_in_load > 0 {
                self.say(
                    den + vec2(78.0, -30.0),
                    "Honey! My cough... thank you, big cub.",
                    HONEY_LIGHT,
                    3.0,
                );
            } else if self.rng.rand_float() > 0.5 {
                self.say(
                    den + vec2(-104.0, -78.0),
                    "Ha! Into the store with it.",
                    CREAM,
                    2.2,
                );
            }

            if self.stored >= self.quota {
                self.best_day = self.best_day.max(self.day);
                for _ in 0..3 {
                    self.sparkle(den + vec2(0.0, 6.0), CREAM, 10);
                }
                self.say(
                    self.den_center() + vec2(-104.0, -78.0),
                    self.day_clear_line(),
                    CREAM,
                    4.0,
                );
                if self.day >= WIN_DAY {
                    // The last day is the finish line: everybody sleeps.
                    self.sfx.request(SFX_WIN);
                    self.begin_cut(Self::epilogue_panels(), AfterCut::Win);
                } else {
                    self.sfx.request(SFX_DAY_CLEAR);
                    self.phase = Phase::Choosing;
                    self.offer_boons();
                }
                return;
            }
        }

        // ---- Foes ----
        self.update_foes(dt);

        // ---- Bumps ----
        if self.stun <= 0.0 {
            let mut hit: Option<usize> = None;
            for (i, foe) in self.foes.iter().enumerate() {
                if foe.pos.distance(self.cub) < foe.reach() + CUB_RADIUS {
                    hit = Some(i);
                    break;
                }
            }
            if let Some(i) = hit {
                self.apply_bump(i);
            }
        }

        // ---- Dusk ----
        if self.day_time <= 0.0 && self.phase == Phase::Playing {
            self.phase = Phase::RunOver;
            self.best_day = self.best_day.max(self.day);
            self.sfx.request(SFX_OVER);
            self.over_reason = format!(
                "Dusk caught you with {} of {} food stored. Mama curled up alone.",
                self.stored, self.quota
            );
        }
    }

    fn update_foes(&mut self, dt: f32) {
        let cub = self.cub;
        let day = self.day as f32;

        for foe in &mut self.foes {
            foe.anim += dt;
            let to_cub = cub - foe.pos;
            let dist = to_cub.length();

            match foe.kind {
                FoeKind::Badger => {
                    foe.cooldown = (foe.cooldown - dt).max(0.0);
                    let speed = 164.0 + day * 4.0;
                    if foe.cooldown > 0.0 {
                        foe.pos -= to_cub.normalize_or_zero() * speed * 1.15 * dt;
                    } else if dist < 470.0 {
                        foe.pos += to_cub.normalize_or_zero() * speed * dt;
                    } else {
                        let home = foe.home - foe.pos;
                        if home.length() > 26.0 {
                            foe.pos += home.normalize_or_zero() * speed * 0.4 * dt;
                        }
                    }
                    if to_cub.x.abs() > 1.0 {
                        foe.facing = to_cub.x.signum();
                    }
                }
                FoeKind::Boar => {
                    if foe.windup > 0.0 {
                        foe.windup -= dt;
                        if foe.windup <= 0.0 {
                            foe.charge = 1.15;
                            foe.charge_dir = to_cub.normalize_or_zero();
                        }
                        foe.facing = to_cub.x.signum();
                    } else if foe.charge > 0.0 {
                        foe.charge -= dt;
                        if foe.charge <= 0.0 {
                            // Spend a moment recovering, or it would charge forever.
                            foe.cooldown = 1.5;
                        }
                        foe.pos += foe.charge_dir * 360.0 * dt;
                        // Bounce off the meadow edges and keep going.
                        if foe.pos.x < 40.0 || foe.pos.x > self.size.x - 40.0 {
                            foe.charge_dir.x = -foe.charge_dir.x;
                            foe.pos.x = foe.pos.x.clamp(40.0, self.size.x - 40.0);
                        }
                        if foe.pos.y < DEN_Y + 40.0 || foe.pos.y > self.size.y - 40.0 {
                            foe.charge_dir.y = -foe.charge_dir.y;
                            foe.pos.y = foe.pos.y.clamp(DEN_Y + 40.0, self.size.y - 40.0);
                        }
                    } else {
                        foe.cooldown = (foe.cooldown - dt).max(0.0);
                        if foe.cooldown <= 0.0 && dist < 340.0 {
                            foe.windup = 0.75;
                            foe.facing = to_cub.x.signum();
                        } else if dist > 300.0 {
                            let home = foe.home - foe.pos;
                            if home.length() > 30.0 {
                                foe.pos += home.normalize_or_zero() * 42.0 * dt;
                            }
                            foe.facing = if home.x.abs() > 1.0 {
                                home.x.signum()
                            } else {
                                foe.facing
                            };
                        }
                    }
                }
                FoeKind::Bee => {
                    foe.cooldown = (foe.cooldown - dt).max(0.0);
                    if foe.cooldown > 0.0 || dist < BEE_HOME_RADIUS {
                        if foe.cooldown <= 0.0 {
                            foe.cooldown = 1.2;
                        }
                        foe.pos += to_cub.normalize_or_zero() * (196.0 + day * 3.0) * dt;
                        foe.facing = to_cub.x.signum();
                    } else {
                        // Drift around the hive in a lazy loop.
                        let angle = foe.anim * 1.4 + foe.home.x * 0.01;
                        let target = foe.home + vec2(angle.cos() * 34.0, angle.sin() * 26.0 - 8.0);
                        foe.pos += (target - foe.pos) * (dt * 2.0).min(1.0);
                        foe.facing = if (target - foe.pos).x > 0.0 {
                            1.0
                        } else {
                            -1.0
                        };
                    }
                }
            }

            foe.pos.x = foe.pos.x.clamp(18.0, self.size.x - 18.0);
            foe.pos.y = foe.pos.y.clamp(DEN_Y + 24.0, self.size.y - 18.0);
        }
    }

    fn apply_bump(&mut self, index: usize) {
        let kind = self.foes[index].kind;
        let from = self.foes[index].pos;

        // Thick Fur: the first bumps of the day are free.
        if self.fur_left > 0 {
            self.fur_left -= 1;
            // Bees read `cooldown` as "angry", so shoo them off instead.
            self.foes[index].cooldown = if kind == FoeKind::Bee { 0.0 } else { 1.6 };
            self.sparkle(self.cub, CREAM, 7);
            self.sfx.request(SFX_FUR);
            self.quip("Mama's fur holds!");
            return;
        }

        let flee_bonus = 0.7 * self.count(Boon::BearStare) as f32;
        let (stun, flee) = match kind {
            FoeKind::Badger => (STUN_TIME, 1.8),
            FoeKind::Bee => (0.55, 1.3),
            FoeKind::Boar => (1.3, 1.2),
        };

        self.stun = stun;
        self.hurt_flash = 0.6;
        self.injury = (self.injury + 1).min(3);
        self.foes[index].cooldown = if kind == FoeKind::Bee {
            0.0
        } else {
            flee + flee_bonus
        };

        // A bee only shakes one thing loose; the bigger animals take the lot.
        let drop = match kind {
            FoeKind::Bee => 1,
            _ => self.carried.len(),
        };
        let dropped: Vec<Food> = self
            .carried
            .drain(0..drop.min(self.carried.len()))
            .collect();
        for (i, food) in dropped.into_iter().enumerate() {
            let away = (self.cub - from).normalize_or_zero();
            let angle = self.rng.rand_float() * std::f32::consts::TAU;
            let spread = vec2(angle.cos(), angle.sin()) * (30.0 + i as f32 * 18.0);
            self.pickups.push(Pickup {
                pos: self.cub + spread,
                vel: away * (110.0 + i as f32 * 25.0) + spread,
                kind: food,
                wobble: 0.0,
            });
        }

        self.sparkle(self.cub, CUB_BELLY, 8);
        self.sfx.request(SFX_BUMP);
        self.quip(match kind {
            FoeKind::Badger => "Oof! Rude badger!",
            FoeKind::Boar => "Not the tusks!",
            FoeKind::Bee => "Ow ow ow bees!",
        });

        // Knocked back a little, so getting hit moves you in the world.
        let away = (self.cub - from).normalize_or_zero();
        self.cub_vel = away * 210.0;
    }

    /// Particles, speech and idle animation keep going on every screen.
    fn update_effects(&mut self, dt: f32) {
        self.quip_cd = (self.quip_cd - dt).max(0.0);
        self.grab_cd = (self.grab_cd - dt).max(0.0);
        self.mama_hop = (self.mama_hop - dt).max(0.0);
        self.pip_wiggle = (self.pip_wiggle - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt * 1.8).max(0.0);
        let mut toast_done = false;
        if let Some((_, life)) = &mut self.toast {
            *life -= dt;
            toast_done = *life <= 0.0;
        }
        if toast_done {
            self.toast = None;
        }

        for p in &mut self.particles {
            p.pos += p.vel * dt;
            p.vel *= (1.0 - dt * 3.0).clamp(0.0, 1.0);
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        for s in &mut self.speech {
            s.life -= dt;
            s.pos.y -= dt * 4.0;
        }
        self.speech.retain(|s| s.life > 0.0);
    }

    // ---- Drawing --------------------------------------------------

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let size = self.size;
        let den = self.den_center();
        let mut canvas = graphics::Canvas::from_frame(ctx, GRASS);

        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, size.y))
                .color(GRASS),
        );
        canvas.draw(&self.decor_mesh, DrawParam::new());

        // The den glows a little warmer once it is full enough. This is drawn
        // under the mound, so the mouth of the den stays a dark hole.
        let warmth = (self.stored as f32 / self.quota.max(1) as f32).clamp(0.0, 1.0);
        canvas.draw(
            &self.dot_mesh,
            DrawParam::new()
                .dest(den + vec2(0.0, 22.0))
                .scale(vec2(98.0, 48.0))
                .color(Color::new(HONEY.r, HONEY.g, HONEY.b, 0.10 + 0.22 * warmth)),
        );

        // Den, with the family waiting by the door.
        canvas.draw(&self.den_mesh, DrawParam::new().dest(den));

        // Mama, old and patient.
        let mama_hop = (self.mama_hop * 9.0).sin().abs() * 8.0;
        let mama_at = den + vec2(-112.0, 30.0 - mama_hop);
        if let Some(art) = &self.sprites {
            draw_bear_px(&mut canvas, &art.mama, mama_at, 3, 1, 1.6, Color::WHITE);
        } else {
            draw_bear(
                &mut canvas,
                &self.mama_rig,
                mama_at,
                -1.0,
                1.45,
                self.clock * 1.2,
                (self.clock * 0.7).sin() * 0.04,
                false,
                Color::WHITE,
            );
        }

        // Pip, curled by the mouth of the den, shivering a little.
        let shiver = if self.pip_wiggle > 0.0 { 2.4 } else { 0.7 };
        let pip_jitter = vec2((self.clock * 22.0).sin(), (self.clock * 17.0).cos()) * shiver;
        let pip_at = den + vec2(84.0, 44.0) + pip_jitter;
        if let Some(art) = &self.sprites {
            draw_bear_px(
                &mut canvas,
                &art.pip,
                pip_at,
                3,
                1,
                0.9,
                Color::new(1.0, 0.98, 0.93, 1.0),
            );
        } else {
            draw_bear(
                &mut canvas,
                &self.pip_rig,
                pip_at,
                -1.0,
                0.66,
                0.6,
                -0.10,
                true,
                Color::new(0.94, 0.97, 0.90, 1.0),
            );
        }

        // Hives and their honey.
        for hive in &self.hives {
            canvas.draw(&self.hive_mesh, DrawParam::new().dest(hive.pos));
            let angry = hive.angry > 0.0;
            let glow = if angry { HONEY_LIGHT } else { HONEY };
            for i in 0..hive.honey {
                canvas.draw(
                    &self.dot_mesh,
                    DrawParam::new()
                        .dest(hive.pos + vec2(-8.0 + i as f32 * 16.0, 14.0))
                        .scale(vec2(4.0, 4.0))
                        .color(glow),
                );
            }
        }

        // Food on the ground.
        for pickup in &self.pickups {
            let bob = (pickup.wobble).sin() * 1.3;
            canvas.draw(
                &self.dot_mesh,
                DrawParam::new()
                    .dest(pickup.pos + vec2(0.0, 7.0))
                    .scale(vec2(8.0, 3.0))
                    .color(Color::new(0.0, 0.0, 0.0, 0.16)),
            );
            let mesh = match pickup.kind {
                Food::Berry => &self.berry_mesh,
                Food::Honey => &self.honey_mesh,
                Food::Treat => &self.treat_mesh,
            };
            canvas.draw(mesh, DrawParam::new().dest(pickup.pos + vec2(0.0, bob)));
        }

        // ---- Animals ----
        for foe in &self.foes {
            match foe.kind {
                FoeKind::Badger => {
                    let grow = if foe.boss { 2.1 } else { 1.0 };
                    if foe.boss {
                        canvas.draw(
                            &self.dot_mesh,
                            DrawParam::new()
                                .dest(foe.pos + vec2(0.0, 16.0))
                                .scale(vec2(46.0, 14.0))
                                .color(Color::new(0.0, 0.0, 0.0, 0.20)),
                        );
                    }
                    canvas.draw(
                        &self.badger_mesh,
                        DrawParam::new()
                            .dest(foe.pos + vec2(0.0, (foe.anim * 8.0).sin() * 1.4))
                            .scale(vec2(foe.facing * grow, grow))
                            .color(if foe.boss {
                                Color::new(1.0, 0.92, 0.86, 1.0)
                            } else {
                                Color::WHITE
                            }),
                    );
                }
                FoeKind::Boar => {
                    let shake = if foe.windup > 0.0 {
                        vec2((foe.anim * 40.0).sin() * 2.0, 0.0)
                    } else {
                        Vec2::ZERO
                    };
                    let dust = if foe.windup > 0.0 { 1.0 } else { 0.0 };
                    if dust > 0.0 {
                        canvas.draw(
                            &self.dot_mesh,
                            DrawParam::new()
                                .dest(foe.pos + vec2(-foe.facing * 26.0, 14.0))
                                .scale(vec2(16.0, 6.0))
                                .color(Color::new(0.75, 0.68, 0.50, 0.30)),
                        );
                    }
                    canvas.draw(
                        &self.boar_mesh,
                        DrawParam::new()
                            .dest(foe.pos + shake)
                            .scale(vec2(foe.facing, 1.0)),
                    );
                }
                FoeKind::Bee => {
                    let flap = 1.0 + (foe.anim * 22.0).sin() * 0.18;
                    canvas.draw(
                        &self.bee_mesh,
                        DrawParam::new()
                            .dest(foe.pos + vec2(0.0, (foe.anim * 6.0).sin() * 3.0))
                            .scale(vec2(foe.facing, flap))
                            .color(if foe.cooldown > 0.0 {
                                HONEY_LIGHT
                            } else {
                                Color::WHITE
                            }),
                    );
                }
            }
        }

        // ---- The cub ----
        let moving = self.cub_vel.length() > 20.0;
        let blink = self.blinking > 0.0;
        let tilt = if self.stun > 0.0 {
            (self.clock * 24.0).sin() * 0.20
        } else if moving {
            (self.gait * 2.0).sin() * 0.045
        } else {
            (self.clock * 1.6).sin() * 0.02
        };
        let squash = if moving {
            1.0 + (self.gait * 4.0).sin() * 0.035
        } else {
            1.0 + (self.clock * 2.0).sin() * 0.012
        };
        let cub_tint = if self.stun > 0.0 {
            Color::new(1.0, 0.86, 0.82, 1.0)
        } else if self.rush > 0.0 {
            Color::new(1.0, 0.99, 0.90, 1.0)
        } else {
            Color::WHITE
        };

        canvas.draw(
            &self.dot_mesh,
            DrawParam::new()
                .dest(self.cub + vec2(0.0, 10.0))
                .scale(vec2(27.0, 7.5))
                .color(Color::new(0.0, 0.0, 0.0, 0.18)),
        );
        if let Some(art) = &self.sprites {
            let frame = if moving {
                ((self.anim_time * 7.0) as usize) % 3
            } else {
                1
            };
            let row = if self.stun > 0.0 { 1 } else { self.pose_row };
            draw_bear_px(&mut canvas, &art.cub, self.cub, row, frame, 0.95, cub_tint);
        } else {
            draw_bear(
                &mut canvas,
                &self.cub_rig,
                self.cub,
                self.facing,
                squash,
                self.gait,
                tilt,
                blink,
                cub_tint,
            );
        }

        // The swipe: a quick pale arc in front of the cub.
        if self.swipe_anim > 0.0 {
            let t = (1.0 - self.swipe_anim / 0.20).clamp(0.0, 1.0);
            let reach = SWIPE_RANGE * (0.45 + 0.55 * t);
            canvas.draw(
                &self.dot_mesh,
                DrawParam::new()
                    .dest(self.cub + vec2(self.facing * reach * 0.55, -4.0))
                    .scale(vec2(reach * 0.45, 18.0 * (1.0 - t) + 3.0))
                    .color(Color::new(1.0, 1.0, 0.96, 0.40 * (1.0 - t))),
            );
        }

        // Food riding on your back.
        for (i, food) in self.carried.iter().enumerate() {
            let mesh = match food {
                Food::Berry => &self.berry_mesh,
                Food::Honey => &self.honey_mesh,
                Food::Treat => &self.treat_mesh,
            };
            canvas.draw(
                mesh,
                DrawParam::new()
                    .dest(self.cub + vec2(-self.facing * (4.0 + i as f32 * 11.0), -26.0))
                    .scale(vec2(0.66, 0.66)),
            );
        }

        // ---- Sparkles ----
        for p in &self.particles {
            let fade = (p.life / p.max_life).clamp(0.0, 1.0);
            canvas.draw(
                &self.dot_mesh,
                DrawParam::new()
                    .dest(p.pos)
                    .scale(vec2(p.size, p.size))
                    .color(Color::new(p.color.r, p.color.g, p.color.b, fade)),
            );
        }

        // ---- Speech bubbles ----
        for s in self.speech.iter().filter(|_| self.phase == Phase::Playing) {
            let fade = (s.life / s.max_life).clamp(0.0, 1.0).min(s.life * 3.0);
            self.draw_bubble(&mut canvas, ctx, s.pos, &s.text, s.color, fade)?;
        }

        // ---- HUD ----
        if self.phase != Phase::Title && self.phase != Phase::Cutscene {
            self.draw_hud(&mut canvas, ctx)?;
            self.draw_toast(&mut canvas, ctx)?;
            self.draw_boss_bar(&mut canvas, ctx)?;
            if self.debug {
                self.draw_debug(&mut canvas, ctx)?;
            }
        }

        if self.hurt_flash > 0.0 {
            canvas.draw(
                &graphics::Quad,
                DrawParam::new()
                    .dest_rect(Rect::new(0.0, 0.0, size.x, size.y))
                    .color(Color::new(0.85, 0.22, 0.16, self.hurt_flash * 0.26)),
            );
        }

        match self.phase {
            Phase::Playing => {}
            Phase::Choosing => self.draw_boon_screen(&mut canvas, ctx)?,
            Phase::RunOver => self.draw_over_screen(&mut canvas, ctx)?,
            Phase::Title => self.draw_title(&mut canvas, ctx)?,
            Phase::Cutscene => self.draw_cutscene(&mut canvas, ctx)?,
        }

        canvas.finish(ctx)?;
        Ok(())
    }

    fn draw_bubble(
        &self,
        canvas: &mut graphics::Canvas,
        ctx: &Context,
        at: Vec2,
        text: &str,
        color: Color,
        fade: f32,
    ) -> GameResult {
        let mut label = make_label(text, 15.0);
        label.set_wrap(false);
        let dims = label.measure(ctx)?;
        let w = dims.x + 22.0;
        let h = dims.y + 14.0;
        // Keep bubbles on screen: a line said over the den must not spill off the top.
        let x = (at.x - w * 0.5).clamp(8.0, (self.size.x - w - 8.0).max(8.0));
        let y = (at.y - h).clamp(92.0, (self.size.y - h - 12.0).max(92.0));
        let rect = Rect::new(x, y, w, h);
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(rect)
                .color(Color::new(0.10, 0.08, 0.06, 0.74 * fade)),
        );
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(rect.x, rect.y, 3.0, h))
                .color(Color::new(color.r, color.g, color.b, 0.9 * fade)),
        );
        canvas.draw(
            &label,
            DrawParam::new()
                .dest(vec2(rect.x + 11.0, rect.y + 7.0))
                .color(Color::new(color.r, color.g, color.b, fade)),
        );
        Ok(())
    }

    fn draw_hud(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let size = self.size;

        // Daylight bar.
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, 9.0))
                .color(Color::new(0.08, 0.08, 0.10, 0.45)),
        );
        let frac = (self.day_time / self.day_len.max(0.01)).clamp(0.0, 1.0);
        let bar = lerp_color(SUNSET, CREAM, frac);
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x * frac, 9.0))
                .color(bar),
        );

        draw_label(
            canvas,
            &format!("DAY {}", self.day),
            32.0,
            CREAM,
            vec2(28.0, 24.0),
        )?;
        draw_label(
            canvas,
            &format!(
                "{} / {} stored  ·  best day {}",
                self.stored, self.quota, self.best_day
            ),
            15.0,
            Color::new(0.93, 0.99, 0.89, 0.76),
            vec2(30.0, 60.0),
        )?;

        let secs = self.day_time.ceil() as u32;
        let clock = format!("{}:{:02}", secs / 60, secs % 60);
        draw_label_right(canvas, ctx, &clock, 32.0, bar, vec2(size.x - 28.0, 24.0))?;
        draw_label_right(
            canvas,
            ctx,
            &format!("carrying {}/{}", self.carried.len(), self.carry_limit()),
            15.0,
            Color::new(0.98, 0.93, 0.88, 0.76),
            vec2(size.x - 30.0, 60.0),
        )?;

        self.draw_hearts(canvas, ctx)?;

        // List of what the run has given you so far.
        if !self.boons.is_empty() {
            let mut counts: Vec<(Boon, usize)> = Vec::new();
            for boon in &self.boons {
                match counts.iter_mut().find(|(b, _)| b == boon) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((*boon, 1)),
                }
            }
            let text: Vec<String> = counts
                .iter()
                .map(|(b, n)| {
                    if *n > 1 {
                        format!("{} x{}", b.name(), n)
                    } else {
                        b.name().to_string()
                    }
                })
                .collect();
            draw_label(
                canvas,
                &text.join("   ·   "),
                13.0,
                Color::new(0.95, 0.93, 0.82, 0.55),
                vec2(30.0, size.y - 26.0),
            )?;
        }

        if self.phase == Phase::Playing && self.day_time > self.day_len - 4.0 {
            let alpha = ((self.day_len - self.day_time) / 2.0).clamp(0.0, 1.0) * 0.85;
            draw_centered_label(
                canvas,
                ctx,
                "WASD / arrows to walk  ·  scoop berries  ·  store them in the den  ·  M to hush",
                14.0,
                Color::new(1.0, 1.0, 0.94, alpha),
                vec2(size.x * 0.5, size.y - 54.0),
            )?;
        }
        Ok(())
    }

    fn draw_boon_screen(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let size = self.size;
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, size.y))
                .color(Color::new(0.08, 0.10, 0.08, 0.80)),
        );

        draw_centered_label(
            canvas,
            ctx,
            &format!("Day {} survived", self.day),
            42.0,
            CREAM,
            vec2(size.x * 0.5, 108.0),
        )?;
        draw_centered_label(
            canvas,
            ctx,
            "Pip is warm for one more night. Choose what kind of bear you'll be.",
            16.0,
            Color::new(0.93, 0.93, 0.86, 0.75),
            vec2(size.x * 0.5, 150.0),
        )?;

        let card_w = 280.0;
        let card_h = 210.0;
        let gap = 26.0;
        let total = card_w * 3.0 + gap * 2.0;
        let left = (size.x - total) * 0.5;
        let top = 210.0;

        for (i, boon) in self.offers.iter().enumerate() {
            let x = left + i as f32 * (card_w + gap);
            let rect = Rect::new(x, top, card_w, card_h);
            canvas.draw(
                &graphics::Quad,
                DrawParam::new()
                    .dest_rect(rect)
                    .color(Color::new(0.16, 0.19, 0.15, 0.96)),
            );
            canvas.draw(
                &graphics::Quad,
                DrawParam::new()
                    .dest_rect(Rect::new(x, top, card_w, 5.0))
                    .color(HONEY),
            );
            draw_label(
                canvas,
                &format!("{}", i + 1),
                22.0,
                HONEY,
                vec2(x + 18.0, top + 18.0),
            )?;
            draw_label(canvas, boon.name(), 24.0, CREAM, vec2(x + 18.0, top + 52.0))?;

            let mut blurb = make_label(boon.blurb(), 14.0);
            blurb.set_wrap(true);
            blurb.set_bounds(vec2(card_w - 36.0, card_h - 80.0));
            canvas.draw(
                &blurb,
                DrawParam::new()
                    .dest(vec2(x + 18.0, top + 92.0))
                    .color(Color::new(0.88, 0.90, 0.84, 0.85)),
            );

            let owned = self.count(*boon);
            if owned > 0 {
                draw_label(
                    canvas,
                    &format!("already have {owned}"),
                    12.0,
                    Color::new(0.98, 0.85, 0.55, 0.7),
                    vec2(x + 18.0, top + card_h - 30.0),
                )?;
            }
        }

        draw_centered_label(
            canvas,
            ctx,
            "press 1, 2 or 3",
            18.0,
            Color::new(0.98, 0.86, 0.55, 0.9),
            vec2(size.x * 0.5, top + card_h + 46.0),
        )?;
        Ok(())
    }

    fn draw_over_screen(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let size = self.size;
        let wash = if self.won {
            Color::new(0.11, 0.09, 0.07, 0.86)
        } else {
            Color::new(0.10, 0.07, 0.08, 0.84)
        };
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, size.y))
                .color(wash),
        );
        let middle = vec2(size.x * 0.5, size.y * 0.5);

        // A few slow flakes, only on the good ending.
        if self.won {
            for i in 0..7 {
                let drift = (self.clock * 0.5 + i as f32 * 1.7).sin() * 14.0;
                let fall = (self.clock * 9.0 + i as f32 * 61.0) % (size.y - 120.0);
                canvas.draw(
                    &self.dot_mesh,
                    DrawParam::new()
                        .dest(vec2(
                            60.0 + i as f32 * (size.x - 120.0) / 7.0 + drift,
                            100.0 + fall,
                        ))
                        .scale(vec2(2.6, 2.6))
                        .color(Color::new(1.0, 1.0, 0.98, 0.55)),
                );
            }
        }

        draw_centered_label(
            canvas,
            ctx,
            if self.won {
                "The long sleep"
            } else {
                "The first snow"
            },
            52.0,
            if self.won { HONEY_LIGHT } else { CREAM },
            middle - vec2(0.0, 96.0),
        )?;

        let reason = if self.won {
            let span = if self.day == 1 {
                "1 day".to_string()
            } else {
                format!("{} days", self.day)
            };
            format!("{span} of carrying. The den is full and Pip's cough has gone quiet.")
        } else {
            self.over_reason.clone()
        };
        draw_centered_label(
            canvas,
            ctx,
            &reason,
            17.0,
            Color::new(0.95, 0.92, 0.86, 0.86),
            middle - vec2(0.0, 40.0),
        )?;

        if self.won {
            draw_centered_label(
                canvas,
                ctx,
                "Mama curled around you both, and the world went soft and white.",
                18.0,
                CREAM,
                middle,
            )?;
        }

        draw_centered_label(
            canvas,
            ctx,
            &format!(
                "Reached day {}  ·  {} food stored across the run",
                self.day, self.total_food
            ),
            20.0,
            HONEY_LIGHT,
            middle + vec2(0.0, if self.won { 46.0 } else { 14.0 }),
        )?;
        draw_centered_label(
            canvas,
            ctx,
            &format!("best day: {}   ·   runs: {}", self.best_day, self.runs),
            15.0,
            Color::new(0.93, 0.90, 0.84, 0.6),
            middle + vec2(0.0, if self.won { 78.0 } else { 48.0 }),
        )?;
        draw_centered_label(
            canvas,
            ctx,
            "R to start a new run  ·  Escape to quit",
            21.0,
            SUNSET,
            middle + vec2(0.0, if self.won { 132.0 } else { 108.0 }),
        )?;
        Ok(())
    }
    // ---- Story -----------------------------------------------------

    fn intro_panels() -> Vec<Cut> {
        vec![
            Cut {
                who: "MAMA",
                line: "You were born in this hollow, little one. So was I, and so was my mother.",
                tint: CREAM,
            },
            Cut {
                who: "MAMA",
                line: "Four generations of us have slept in this dirt. It has never once let us down.",
                tint: CREAM,
            },
            Cut {
                who: "PIP",
                line: "Mama... is it always this cold before the snow?",
                tint: PIP_BELLY,
            },
            Cut {
                who: "MAMA",
                line: "Always. That is why we fill it. Berries in the walls, honey at the back, warmth in the middle.",
                tint: CREAM,
            },
            Cut {
                who: "MAMA",
                line: "I cannot walk the ridge anymore. My hip remembers a winter it did not survive.",
                tint: CREAM,
            },
            Cut {
                who: "PIP",
                line: "And I cannot walk at all. My chest rattles like a dry leaf in the wind.",
                tint: PIP_BELLY,
            },
            Cut {
                who: "YOU",
                line: "Then I will carry it. Every berry between here and the ridge. All of it.",
                tint: HONEY_LIGHT,
            },
            Cut {
                who: "MAMA",
                line: "Then go, small one. And come back before dusk. Always come back.",
                tint: CREAM,
            },
        ]
    }

    fn interlude_panels() -> Vec<Cut> {
        vec![
            Cut {
                who: "MAMA",
                line: "The old badger has been watching you. He remembers the year the berries never came.",
                tint: CREAM,
            },
            Cut {
                who: "MAMA",
                line: "He took this den's mouth from my mother. I took it back with these teeth.",
                tint: CREAM,
            },
            Cut {
                who: "PIP",
                line: "Mama, you are scaring me.",
                tint: PIP_BELLY,
            },
            Cut {
                who: "MAMA",
                line: "Good. Be scared, and be quick. You do not have to be kind to him. Only to each other.",
                tint: CREAM,
            },
        ]
    }

    fn epilogue_panels() -> Vec<Cut> {
        vec![
            Cut {
                who: "MAMA",
                line: "Ten days. I sent a cub out for berries and a bear came back.",
                tint: CREAM,
            },
            Cut {
                who: "PIP",
                line: "My chest does not rattle. Listen. It does not rattle.",
                tint: PIP_BELLY,
            },
            Cut {
                who: "MAMA",
                line: "Come in out of the snow. All of you. The den is full and the wind can do what it likes.",
                tint: CREAM,
            },
            Cut {
                who: "YOU",
                line: "I will keep watch a little longer. Just until the last of the light goes.",
                tint: HONEY_LIGHT,
            },
            Cut {
                who: "MAMA",
                line: "You will not. You will sleep, and you will be small again for a few months. That is the deal.",
                tint: CREAM,
            },
        ]
    }

    fn begin_cut(&mut self, panels: Vec<Cut>, after: AfterCut) {
        self.cut = panels;
        self.cut_i = 0;
        self.cut_after = after;
        if self.cut.is_empty() {
            self.apply_cut_after();
        } else {
            self.phase = Phase::Cutscene;
            self.sfx.request(SFX_CHOOSE);
        }
    }

    fn advance_cut(&mut self) {
        if self.cut_i + 1 < self.cut.len() {
            self.cut_i += 1;
            self.sfx.request(SFX_PICKUP);
        } else {
            self.apply_cut_after();
        }
    }

    fn apply_cut_after(&mut self) {
        match self.cut_after {
            AfterCut::Resume => self.phase = Phase::Playing,
            AfterCut::Win => {
                self.won = true;
                self.phase = Phase::RunOver;
            }
        }
    }

    // ---- Input per phase -------------------------------------------

    fn title_keys(&mut self, input: KeyInput) {
        match input.event.physical_key {
            PhysicalKey::Code(KeyCode::Digit1) => self.title_page = 0,
            PhysicalKey::Code(KeyCode::Digit2) => self.title_page = 1,
            PhysicalKey::Code(KeyCode::Digit3) => self.title_page = 2,
            PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter) => {
                self.title_page = 0;
                self.start_run();
                self.begin_cut(Self::intro_panels(), AfterCut::Resume);
            }
            _ => {}
        }
    }

    fn playing_keys(&mut self, input: KeyInput) {
        match input.event.physical_key {
            PhysicalKey::Code(KeyCode::Space) => self.do_swipe(),
            PhysicalKey::Code(KeyCode::KeyC) => self.cuddle(),
            PhysicalKey::Code(KeyCode::KeyN) => self.nurse(),
            PhysicalKey::Code(KeyCode::F1) => {
                self.debug = !self.debug;
                let line = if self.debug { "debug on" } else { "debug off" };
                self.toast_line(line);
            }
            _ => {}
        }
        if let Key::Character(text) = &input.event.logical_key {
            for ch in text.chars() {
                if ch.is_ascii_alphabetic() {
                    self.cheat_buf.push(ch.to_ascii_lowercase());
                }
            }
            while self.cheat_buf.len() > 24 {
                self.cheat_buf.remove(0);
            }
            self.check_cheats();
        }
    }

    // ---- Comfort: the whole point of a bear cub --------------------

    fn cuddle(&mut self) {
        if self.cuddle_cd > 0.0 {
            return;
        }
        if self.cub.distance(self.den_center()) > DEN_REACH + 46.0 {
            self.toast_line("Mama is by the den. Go to her.");
            return;
        }
        self.cuddle_cd = 2.2;
        self.bond_mama = (self.bond_mama + 1.0).min(9.0);
        self.cuddle_streak += 1;
        let hurt = self.injury > 0;
        self.injury = self.injury.saturating_sub(1);
        self.mama_hop = 0.8;
        self.sfx.request(SFX_FUR);
        self.sparkle(self.cub, CUB_EAR, 8);
        let line = if self.cuddle_streak >= 3 {
            "Mama hums the den-song. You have not heard it since you were smaller than a berry."
        } else if hurt {
            "Mama licks the bruise. It still hurts, but less."
        } else if self.bond_mama >= CLOSE_BOND {
            "You fit under her chin exactly. You always have."
        } else {
            "Mama presses her nose to your ear. Warm."
        };
        self.say(self.cub + vec2(0.0, -58.0), line, CREAM, 2.8);
    }

    fn nurse(&mut self) {
        if self.cub.distance(self.den_center()) > DEN_REACH + 46.0 {
            self.toast_line("Pip is the small pale one, by the den.");
            return;
        }
        let pos = self
            .carried
            .iter()
            .position(|f| *f == Food::Honey)
            .or_else(|| self.carried.iter().position(|f| *f == Food::Treat))
            .or_else(|| self.carried.iter().position(|f| *f == Food::Berry));
        let Some(i) = pos else {
            self.toast_line("Nothing to give him. Go and find something.");
            return;
        };
        let food = self.carried.remove(i);
        let gain = if food == Food::Berry { 1.0 } else { 1.5 };
        self.bond_pip = (self.bond_pip + gain).min(9.0);
        self.injury = 0;
        self.pip_wiggle = 1.6;
        self.sfx.request(SFX_HONEY);
        self.sparkle(self.cub + vec2(0.0, -18.0), HONEY_LIGHT, 9);
        let line = match food {
            Food::Honey => {
                "Pip drinks the honey slowly, the way you are supposed to. His cough goes quiet."
            }
            Food::Treat => "Pip's eyes go wide. He eats the ribbon too. You pretend not to see.",
            _ => "Pip chews the mashed berries. Not honey, but it is warm going down.",
        };
        self.say(self.cub + vec2(0.0, -58.0), line, PIP_BELLY, 3.2);
    }

    // ---- Combat ----------------------------------------------------

    fn do_swipe(&mut self) {
        if self.swipe_cd > 0.0 {
            return;
        }
        self.swipe_cd = SWIPE_CD;
        self.swipe_anim = 0.20;
        self.sfx.request(SFX_BUMP);
        let here = self.cub;
        let front = self.facing;
        let mut strikes: Vec<usize> = Vec::new();
        for (i, foe) in self.foes.iter().enumerate() {
            let to = foe.pos - here;
            let in_front = to.x * front > -14.0 || to.length() < 30.0;
            if to.length() < SWIPE_RANGE + foe.reach() && in_front {
                strikes.push(i);
            }
        }
        if strikes.is_empty() {
            self.sparkle(here + vec2(front * 34.0, -6.0), CREAM, 3);
            return;
        }
        let mut down: Vec<usize> = Vec::new();
        for i in &strikes {
            self.foes[*i].hp -= 1;
            self.foes[*i].cooldown = 1.3;
            let push = (self.foes[*i].pos - here).normalize_or_zero() * 44.0;
            self.foes[*i].pos += push;
            let at = self.foes[*i].pos;
            self.sparkle(at, CREAM, 5);
            if self.foes[*i].hp <= 0 {
                down.push(*i);
            }
        }
        for i in down.into_iter().rev() {
            let foe = self.foes.remove(i);
            self.bump_kill(foe);
        }
        self.quip("Out of my meadow.");
    }

    fn bump_kill(&mut self, foe: Foe) {
        if foe.boss {
            self.boss_down = true;
            self.stored += 4;
            self.total_food += 4;
            self.sfx.request(SFX_DAY_CLEAR);
            self.say(
                self.den_center() + vec2(-104.0, -78.0),
                "The old badger will not come back. Not this winter.",
                CREAM,
                4.0,
            );
            self.sparkle(foe.pos, HONEY_LIGHT, 24);
            self.toast_line("The old badger is gone. +4 stored.");
            self.mama_hop = 1.0;
        } else {
            self.sparkle(foe.pos, BADGER_PALE, 12);
            if self.rng.rand_float() > 0.55 {
                self.pickups.push(Pickup {
                    pos: foe.pos,
                    vel: Vec2::ZERO,
                    kind: Food::Honey,
                    wobble: 0.0,
                });
            }
        }
    }

    // ---- Cheats and easter eggs ------------------------------------

    fn toast_line(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), 2.6));
    }

    fn check_cheats(&mut self) {
        let buf = self.cheat_buf.clone();
        if buf.contains("honey") {
            self.cheat_buf.clear();
            for i in 0..6 {
                let angle = i as f32 / 6.0 * std::f32::consts::TAU;
                self.pickups.push(Pickup {
                    pos: self.cub + vec2(angle.cos() * 46.0, angle.sin() * 40.0),
                    vel: Vec2::ZERO,
                    kind: Food::Honey,
                    wobble: 0.0,
                });
            }
            self.sfx.request(SFX_HONEY);
            self.toast_line("cheat: honey for everyone");
        } else if buf.contains("smasher") || buf.contains("adam") {
            self.cheat_buf.clear();
            self.spawn_adam();
        } else if buf.contains("cuddle") {
            self.cheat_buf.clear();
            self.bond_mama = 9.0;
            self.bond_pip = 9.0;
            self.toast_line("cheat: very loved");
        } else if buf.contains("quota") {
            self.cheat_buf.clear();
            self.stored = self.quota;
            self.toast_line("cheat: quota filled - deliver something to end the day");
        }
    }

    fn spawn_adam(&mut self) {
        if self.adam {
            self.toast_line("he is already here");
            return;
        }
        self.adam = true;
        let pos = vec2(self.size.x - 120.0, self.size.y - 120.0);
        self.foes.push(Foe {
            kind: FoeKind::Badger,
            pos,
            home: pos,
            cooldown: 0.0,
            windup: 0.0,
            charge: 0.0,
            charge_dir: Vec2::ZERO,
            facing: -1.0,
            anim: 0.0,
            hp: 16,
            boss: true,
        });
        self.toast_line("easter egg: something chrome is coming out of the treeline");
        self.say(
            self.den_center() + vec2(0.0, -84.0),
            "Wake up, cub. We have a den to take.",
            Color::new(0.86, 0.92, 1.0, 1.0),
            4.5,
        );
    }

    fn ambient_chat(&mut self, dt: f32) {
        if self.day_time > self.day_len - 3.0 {
            return;
        }
        self.chat_cd -= dt;
        if self.chat_cd > 0.0 {
            return;
        }
        self.chat_cd = 12.0 + self.rng.rand_float() * 8.0;
        let line = CHATTER[self.chat_i % CHATTER.len()];
        self.chat_i += 1;
        let from_pip = self.chat_i % 3 == 0;
        let (at, colour) = if from_pip {
            (self.den_center() + vec2(84.0, 26.0), PIP_BELLY)
        } else {
            (self.den_center() + vec2(-112.0, 8.0), CREAM)
        };
        self.say(at, line, colour, 3.6);
    }

    // ---- New screens -----------------------------------------------

    fn heart_row(&self, bond: f32) -> String {
        let full = ((bond / 3.0).ceil() as i32).clamp(0, 3);
        let mut out = String::new();
        for i in 0..3 {
            out.push(if i < full { 'o' } else { '-' });
        }
        out
    }

    fn draw_hearts(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let text = format!(
            "MAMA {}   PIP {}   {}",
            self.heart_row(self.bond_mama),
            self.heart_row(self.bond_pip),
            if self.injury > 0 {
                format!("bruised x{}", self.injury)
            } else {
                "unhurt".to_string()
            }
        );
        draw_label_right(
            canvas,
            ctx,
            &text,
            13.0,
            Color::new(0.98, 0.88, 0.92, 0.72),
            vec2(self.size.x - 30.0, 82.0),
        )?;
        Ok(())
    }

    fn draw_toast(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let Some((text, life)) = &self.toast else {
            return Ok(());
        };
        let alpha = (life / 2.6).clamp(0.0, 1.0);
        draw_centered_label(
            canvas,
            ctx,
            text,
            15.0,
            Color::new(0.99, 0.92, 0.62, alpha),
            vec2(self.size.x * 0.5, self.size.y - 90.0),
        )?;
        Ok(())
    }

    fn draw_title(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let size = self.size;
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, size.y))
                .color(Color::new(0.05, 0.08, 0.06, 0.74)),
        );
        let mid = vec2(size.x * 0.5, 0.0);

        draw_centered_label(
            canvas,
            ctx,
            "BEAR CUB BERRIES",
            54.0,
            HONEY_LIGHT,
            mid + vec2(0.0, 92.0),
        )?;
        draw_centered_label(
            canvas,
            ctx,
            "a small roguelite about a very small bear",
            17.0,
            CREAM,
            mid + vec2(0.0, 134.0),
        )?;

        match self.title_page {
            0 => {
                draw_centered_label(
                    canvas,
                    ctx,
                    "SPACE  begin the winter",
                    22.0,
                    SUNSET,
                    mid + vec2(0.0, 236.0),
                )?;
                draw_centered_label(
                    canvas,
                    ctx,
                    "2  credits        3  cheats & controls",
                    15.0,
                    Color::new(0.92, 0.92, 0.86, 0.70),
                    mid + vec2(0.0, 280.0),
                )?;
            }
            1 => {
                for (i, line) in [
                    "bear sprites - the three 3x4 sheets you dropped in",
                    "sound - Arcade Sound FX pack",
                    "engine - ggez 0.10",
                    "everything else - one file, two hands",
                ]
                .iter()
                .enumerate()
                {
                    draw_centered_label(
                        canvas,
                        ctx,
                        line,
                        16.0,
                        if i == 0 { HONEY_LIGHT } else { CREAM },
                        mid + vec2(0.0, 220.0 + i as f32 * 34.0),
                    )?;
                }
                draw_centered_label(
                    canvas,
                    ctx,
                    "1  back",
                    15.0,
                    Color::new(0.92, 0.92, 0.86, 0.70),
                    mid + vec2(0.0, 380.0),
                )?;
            }
            _ => {
                for (i, line) in [
                    "WASD / arrows   walk on all fours",
                    "SPACE           swipe",
                    "C               cuddle Mama (heals bruises)",
                    "N               nurse Pip (feed him from your paws)",
                    "M               hush the sound      F1   debug",
                    "typing: honey - cuddle - quota - smasher",
                ]
                .iter()
                .enumerate()
                {
                    draw_centered_label(
                        canvas,
                        ctx,
                        line,
                        15.0,
                        if i == 5 { HONEY_LIGHT } else { CREAM },
                        mid + vec2(0.0, 210.0 + i as f32 * 32.0),
                    )?;
                }
                draw_centered_label(
                    canvas,
                    ctx,
                    "1  back",
                    15.0,
                    Color::new(0.92, 0.92, 0.86, 0.70),
                    mid + vec2(0.0, 420.0),
                )?;
            }
        }
        Ok(())
    }

    fn draw_cutscene(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let size = self.size;
        let Some(panel) = self.cut.get(self.cut_i) else {
            return Ok(());
        };
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, size.y))
                .color(Color::new(0.04, 0.05, 0.05, 0.72)),
        );
        // Letterbox, because this is a story now.
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, 0.0, size.x, 64.0))
                .color(Color::new(0.02, 0.02, 0.02, 0.92)),
        );
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(0.0, size.y - 58.0, size.x, 58.0))
                .color(Color::new(0.02, 0.02, 0.02, 0.92)),
        );

        draw_label(
            canvas,
            panel.who,
            20.0,
            panel.tint,
            vec2(size.x * 0.5 - 250.0, size.y * 0.30),
        )?;
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(
                    size.x * 0.5 - 250.0,
                    size.y * 0.30 + 30.0,
                    60.0,
                    2.0,
                ))
                .color(panel.tint),
        );

        let mut body = make_label(panel.line, 19.0);
        body.set_wrap(true);
        body.set_bounds(vec2(size.x - 260.0, 260.0));
        canvas.draw(
            &body,
            DrawParam::new()
                .dest(vec2(size.x * 0.5 - 250.0, size.y * 0.30 + 48.0))
                .color(panel.tint),
        );

        draw_label_right(
            canvas,
            ctx,
            &format!("{}/{}   SPACE", self.cut_i + 1, self.cut.len()),
            14.0,
            Color::new(0.90, 0.88, 0.82, 0.72),
            vec2(size.x - 40.0, size.y - 34.0),
        )?;
        Ok(())
    }

    fn draw_boss_bar(&self, canvas: &mut graphics::Canvas, ctx: &Context) -> GameResult {
        let Some(boss) = self.foes.iter().find(|f| f.boss) else {
            return Ok(());
        };
        let full = if boss.hp > 8 { 16.0 } else { 8.0 };
        let frac = (boss.hp as f32 / full).clamp(0.0, 1.0);
        let w = 320.0;
        let x = (self.size.x - w) * 0.5;
        let y = 46.0;
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(x, y, w, 12.0))
                .color(Color::new(0.05, 0.04, 0.04, 0.55)),
        );
        canvas.draw(
            &graphics::Quad,
            DrawParam::new()
                .dest_rect(Rect::new(x, y, w * frac, 12.0))
                .color(BADGER_DARK),
        );
        draw_centered_label(
            canvas,
            ctx,
            "THE OLD BADGER",
            13.0,
            Color::new(0.98, 0.92, 0.88, 0.85),
            vec2(self.size.x * 0.5, y - 10.0),
        )?;
        Ok(())
    }

    fn draw_debug(&self, canvas: &mut graphics::Canvas, _ctx: &Context) -> GameResult {
        let phase = match self.phase {
            Phase::Title => "title",
            Phase::Cutscene => "cutscene",
            Phase::Playing => "playing",
            Phase::Choosing => "choosing",
            Phase::RunOver => "over",
        };
        let lines = [
            format!("phase {phase}  day {}  {}s", self.day, self.day_time as i32),
            format!(
                "cub {:.0},{:.0}  carried {}  stored {}/{}",
                self.cub.x,
                self.cub.y,
                self.carried.len(),
                self.stored,
                self.quota
            ),
            format!(
                "bond m{:.0} p{:.0}  injury {}  foes {}",
                self.bond_mama,
                self.bond_pip,
                self.injury,
                self.foes.len()
            ),
        ];
        for (i, line) in lines.iter().enumerate() {
            draw_label(
                canvas,
                line,
                12.0,
                Color::new(1.0, 1.0, 0.7, 0.9),
                vec2(18.0, self.size.y - 96.0 + i as f32 * 16.0),
            )?;
        }
        Ok(())
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.update(ctx);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.draw(ctx)
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, repeat: bool) -> GameResult {
        if repeat {
            return Ok(());
        }
        if input.event.logical_key == Key::Named(NamedKey::Escape) {
            ctx.request_quit();
            return Ok(());
        }

        if input.event.physical_key == PhysicalKey::Code(KeyCode::KeyM) {
            self.sfx.muted = !self.sfx.muted;
            let line = if self.sfx.muted {
                "Ears down."
            } else {
                "I hear everything again."
            };
            self.say(self.cub + vec2(0.0, -58.0), line, CREAM, 1.4);
            return Ok(());
        }

        match self.phase {
            Phase::Choosing => {
                let choice = match input.event.physical_key {
                    PhysicalKey::Code(KeyCode::Digit1) => Some(0),
                    PhysicalKey::Code(KeyCode::Digit2) => Some(1),
                    PhysicalKey::Code(KeyCode::Digit3) => Some(2),
                    _ => None,
                };
                if let Some(i) = choice {
                    self.take_boon(i);
                }
            }
            Phase::RunOver => {
                if input.event.physical_key == PhysicalKey::Code(KeyCode::KeyR) {
                    self.start_run();
                    self.begin_cut(Self::intro_panels(), AfterCut::Resume);
                }
            }
            Phase::Playing => {
                self.playing_keys(input);
            }
            Phase::Title => {
                self.title_keys(input);
            }
            Phase::Cutscene => {
                if matches!(
                    input.event.physical_key,
                    PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter)
                ) {
                    self.advance_cut();
                }
            }
        }
        Ok(())
    }
}

// *********************************************************************
// Helpers.
// *********************************************************************

impl Pickup {
    fn kind_color(&self) -> Color {
        match self.kind {
            Food::Berry => BERRY_LIGHT,
            Food::Honey => HONEY_LIGHT,
            Food::Treat => CREAM,
        }
    }
}

fn held(ctx: &Context, code: KeyCode) -> bool {
    ctx.keyboard
        .is_physical_key_pressed(&PhysicalKey::Code(code))
}

fn dim(color: Color, factor: f32) -> Color {
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

fn make_label(text: &str, scale: f32) -> Text {
    let mut label = Text::new(text);
    label.set_font("LiberationMono");
    label.set_scale(scale);
    label.set_wrap(false);
    label
}

fn draw_label(
    canvas: &mut graphics::Canvas,
    text: &str,
    scale: f32,
    color: Color,
    pos: Vec2,
) -> GameResult {
    let label = make_label(text, scale);
    canvas.draw(&label, DrawParam::new().dest(pos).color(color));
    Ok(())
}

fn draw_label_right(
    canvas: &mut graphics::Canvas,
    ctx: &Context,
    text: &str,
    scale: f32,
    color: Color,
    right_edge: Vec2,
) -> GameResult {
    let label = make_label(text, scale);
    let width = label.measure(ctx)?.x;
    canvas.draw(
        &label,
        DrawParam::new()
            .dest(right_edge - vec2(width, 0.0))
            .color(color),
    );
    Ok(())
}

fn draw_centered_label(
    canvas: &mut graphics::Canvas,
    ctx: &Context,
    text: &str,
    scale: f32,
    color: Color,
    center: Vec2,
) -> GameResult {
    let label = make_label(text, scale);
    let dims = label.measure(ctx)?;
    canvas.draw(
        &label,
        DrawParam::new()
            .dest(center - vec2(dims.x * 0.5, dims.y * 0.5))
            .color(color),
    );
    Ok(())
}

fn seeded_rng() -> Rand32 {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5EED_1234_ABCD_0001);
    Rand32::new(seed)
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        from.r + (to.r - from.r) * t,
        from.g + (to.g - from.g) * t,
        from.b + (to.b - from.b) * t,
        from.a + (to.a - from.a) * t,
    )
}

fn main() -> GameResult {
    let (mut ctx, event_loop) = ggez::ContextBuilder::new("bear_cubs", "ggez")
        .window_setup(conf::WindowSetup::default().title("Bear Cub Berries"))
        .window_mode(
            conf::WindowMode::default()
                .dimensions(WINDOW_SIZE.0, WINDOW_SIZE.1)
                .resizable(false),
        )
        .add_resource_path({
            // Resources live beside this crate, no matter where the binary is run from.
            let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            dir.push("resources");
            dir
        })
        .build()?;

    let state = Game::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}
