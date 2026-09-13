//! The Bevy app: plugins, resources, and the schedule. The driver runs once
//! per frame in `Update`, before the view reads the snapshot it published.

use crate::camera::{LmbGesture, orbit_camera};
use crate::driver::Driver;
use crate::editor::Editor;
use crate::layout::Layout;
use crate::palette::{CLEAR, Frame, Interface};
use crate::{input, ui, view};
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use std::path::PathBuf;

/// What the player is doing with the mouse (`docs/06`, Q10): nothing, or
/// one of the mark tools.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    /// A building plan for the model.
    Building(String),
    /// A paint plan for the color.
    Paint(String),
    /// An overlay plan for the label.
    Overlay(String),
    /// A plan to clear the slot.
    Clear(sim::PlanKind),
    /// Withdraw the team's plan of that kind.
    Unmark(sim::PlanKind),
}

/// Renderer-side state no system shares with the sim.
#[derive(Resource)]
pub struct ViewState {
    pub tool: Tool,
    pub selected: Option<sim::EntityId>,
    /// The programs directory, re-read on `Shift+D`.
    pub programs: Option<PathBuf>,
    pub deployed: std::collections::BTreeMap<String, sim::Bundle>,
    /// Lines shown in the log window, newest last.
    pub log_lines: Vec<String>,
    /// The windows' placements (Q39), saved beside the programs.
    pub layout: Layout,
    pub layout_path: Option<PathBuf>,
    pub layout_dirty: bool,
    pub layout_saved_at: f32,
    /// The tile under the cursor, if any.
    pub hover: Option<sim::TilePos>,
    pub status: String,
    /// The working copies (`docs/07`, Q33).
    pub editor: Editor,
    /// The fault tick last logged per machine, so each record logs once.
    pub faults_seen: std::collections::HashMap<sim::EntityId, u64>,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            tool: Tool::Select,
            selected: None,
            programs: None,
            deployed: Default::default(),
            log_lines: Vec::new(),
            layout: Layout::default(),
            layout_path: None,
            layout_dirty: false,
            layout_saved_at: 0.0,
            hover: None,
            status: String::new(),
            editor: Editor::default(),
            faults_seen: Default::default(),
        }
    }
}

/// The driver as the app's non-send resource: the sim holds `Rc`s, so it
/// stays on the main thread, which is also where the one system that may
/// touch it runs.
pub struct DriverResource(pub Driver);

/// `RENDER_SCREENSHOT=path`: save a frame of the window to `path` once the
/// scene has settled — at frame `RENDER_SCREENSHOT_FRAME` (default 90), or
/// once the driver reaches tick `RENDER_SCREENSHOT_TICK`, which two peers
/// can share — then exit a second later: a look at the view without a
/// person at the window.
fn screenshot(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    mut frames: Local<u32>,
    mut taken: Local<Option<u32>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(path) = std::env::var("RENDER_SCREENSHOT") else {
        return;
    };
    let var = |name: &str| std::env::var(name).ok().and_then(|f| f.parse::<u64>().ok());
    *frames += 1;
    let due = match var("RENDER_SCREENSHOT_TICK") {
        Some(tick) => driver.0.tick >= tick,
        None => u64::from(*frames) >= var("RENDER_SCREENSHOT_FRAME").unwrap_or(90),
    };
    if due && taken.is_none() {
        *taken = Some(*frames);
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
    if taken.is_some_and(|t| *frames == t + 60) {
        exit.write(AppExit::Success);
    }
}

/// Run one frame of the driver with the wall-clock delta (`docs/06`).
fn drive(time: Res<Time>, mut driver: NonSendMut<DriverResource>, mut state: ResMut<ViewState>) {
    let dt = f64::from(time.delta_secs());
    let before = driver.0.tick;
    driver.0.advance(dt);
    if driver.0.tick != before || !driver.0.events.is_empty() {
        view::collect_log(&mut driver.0, &mut state);
    }
}

pub fn run(
    driver: Driver,
    programs: Option<PathBuf>,
    deployed: std::collections::BTreeMap<String, sim::Bundle>,
) {
    let title = format!("programming game — {}", driver.map_name);
    let frame = Frame {
        min: driver.bounds.0,
        max: driver.bounds.1,
    };
    let interface = Interface::load().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    let editor = Editor::from_bundles(&deployed);
    let layout_path = Layout::path(programs.as_deref());
    let layout = Layout::load(layout_path.as_deref());
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    resolution: (1400, 900).into(),
                    ..default()
                }),
                ..default()
            })
            // The baked textures live beside this crate, wherever the
            // binary runs from.
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(),
                ..default()
            }),
    )
    .add_plugins(EguiPlugin::default())
    .insert_resource(ClearColor(CLEAR))
    .insert_resource(ViewState {
        programs,
        deployed,
        editor,
        layout,
        layout_path,
        ..Default::default()
    })
    .insert_resource(frame)
    .insert_resource(interface)
    .insert_resource(view::Tuning::load())
    .insert_resource(view::Entities::default())
    .insert_resource(LmbGesture::default())
    .insert_non_send(DriverResource(driver))
    .add_systems(Startup, view::setup)
    .add_systems(
        Update,
        (
            drive,
            view::sync_tiles,
            view::sync_machines,
            view::interpolate,
            view::health_bars,
            view::fault_marks,
            view::sounds,
            view::animate,
            orbit_camera,
            input::keys,
            input::click,
            view::markers,
            view::billboard_bars,
            view::level_rings,
            screenshot,
        )
            .chain(),
    )
    .add_systems(EguiPrimaryContextPass, ui::panels);
    app.run();
}
