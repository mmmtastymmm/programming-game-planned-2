//! The Bevy app: plugins, resources, and the schedule. The driver runs once
//! per frame in `Update`, before the view reads the snapshot it published.

use crate::driver::Driver;
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
    /// The programs directory, re-read on `D`.
    pub programs: Option<PathBuf>,
    pub deployed: std::collections::BTreeMap<String, sim::Bundle>,
    /// Lines shown in the log panel, newest last.
    pub log_lines: Vec<String>,
    pub show_log: bool,
    /// The tile under the cursor, if any.
    pub hover: Option<sim::TilePos>,
    pub status: String,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            tool: Tool::Select,
            selected: None,
            programs: None,
            deployed: Default::default(),
            log_lines: Vec::new(),
            show_log: true,
            hover: None,
            status: String::new(),
        }
    }
}

/// The driver as the app's non-send resource: the sim holds `Rc`s, so it
/// stays on the main thread, which is also where the one system that may
/// touch it runs.
pub struct DriverResource(pub Driver);

/// Run one frame of the driver with the wall-clock delta (`docs/06`).
fn drive(time: Res<Time>, mut driver: NonSendMut<DriverResource>, mut state: ResMut<ViewState>) {
    let dt = f64::from(time.delta_secs());
    let before = driver.0.tick;
    driver.0.advance(dt);
    if driver.0.tick != before {
        view::collect_log(&driver.0, &mut state);
    }
}

pub fn run(
    driver: Driver,
    programs: Option<PathBuf>,
    deployed: std::collections::BTreeMap<String, sim::Bundle>,
) {
    let title = format!("programming game — {}", driver.map_name);
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title,
            resolution: (1400, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(EguiPlugin::default())
    .insert_resource(ClearColor(Color::srgb(0.06, 0.06, 0.08)))
    .insert_resource(ViewState {
        programs,
        deployed,
        ..Default::default()
    })
    .insert_resource(view::Entities::default())
    .insert_non_send(DriverResource(driver))
    .add_systems(Startup, view::setup)
    .add_systems(
        Update,
        (
            drive,
            view::sync_tiles,
            view::sync_machines,
            view::interpolate,
            view::sounds,
            input::camera,
            input::keys,
            input::click,
        )
            .chain(),
    )
    .add_systems(EguiPrimaryContextPass, ui::panels);
    app.run();
}
