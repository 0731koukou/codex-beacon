use std::{
    env,
    os::windows::process::CommandExt,
    process::Command,
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewWindow,
};
use windows::Win32::{
    Foundation::{POINT, RECT},
    UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect},
};

const WINDOW_LABEL: &str = "main";
const STAGE_WINDOW_WIDTH: f64 = 820.0;
const STAGE_WINDOW_HEIGHT: f64 = 460.0;
const WINDOW_MARGIN_Y: f64 = 12.0;
const COLLAPSED_ISLAND_WIDTH: f64 = 510.0;
const COLLAPSED_ISLAND_HEIGHT: f64 = 68.0;
const IDLE_ISLAND_WIDTH: f64 = 320.0;
const IDLE_ISLAND_HEIGHT: f64 = 56.0;
const EXPANDED_ISLAND_WIDTH: f64 = 660.0;
const EXPANDED_ISLAND_HEIGHT: f64 = 404.0;
const EXPANDED_RADIUS: f64 = 26.0;
const EDGE_SNAP_THRESHOLD: f64 = 16.0;
const EDGE_SNAP_MARGIN: f64 = 12.0;
const STARTUP_REGISTRY_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_REGISTRY_VALUE: &str = "Codex Beacon";
const LEGACY_STARTUP_REGISTRY_VALUE: &str = "FocuSD Island";
const LATEST_RELEASE_URL: &str = "https://github.com/0731koukou/codex-beacon/releases/latest";
const CREATE_NO_WINDOW: u32 = 0x08000000;

static WINDOW_STATE: OnceLock<Mutex<IslandWindowState>> = OnceLock::new();

#[derive(Clone, Copy)]
enum IslandMode {
    Collapsed,
    Expanded,
}

impl IslandMode {
    fn from_value(value: &str) -> Result<Self, String> {
        match value {
            "collapsed" => Ok(Self::Collapsed),
            "expanded" => Ok(Self::Expanded),
            _ => Err(format!("Unsupported island mode: {value}")),
        }
    }

    fn base_size(self) -> (f64, f64) {
        match self {
            Self::Collapsed => (COLLAPSED_ISLAND_WIDTH, COLLAPSED_ISLAND_HEIGHT),
            Self::Expanded => (EXPANDED_ISLAND_WIDTH, EXPANDED_ISLAND_HEIGHT),
        }
    }

    fn corner_radius(self) -> f64 {
        match self {
            Self::Collapsed => COLLAPSED_ISLAND_HEIGHT / 2.0,
            Self::Expanded => EXPANDED_RADIUS,
        }
    }
}

#[derive(Clone, Copy)]
struct IslandWindowState {
    mode: IslandMode,
    has_task: bool,
}

impl Default for IslandWindowState {
    fn default() -> Self {
        Self {
            mode: IslandMode::Collapsed,
            has_task: false,
        }
    }
}

impl IslandWindowState {
    fn size(self) -> (f64, f64) {
        match (self.mode, self.has_task) {
            (IslandMode::Collapsed, false) => (IDLE_ISLAND_WIDTH, IDLE_ISLAND_HEIGHT),
            _ => self.mode.base_size(),
        }
    }

    fn corner_radius(self) -> f64 {
        match (self.mode, self.has_task) {
            (IslandMode::Collapsed, false) => IDLE_ISLAND_HEIGHT / 2.0,
            _ => self.mode.corner_radius(),
        }
    }
}

#[tauri::command]
pub(crate) fn set_island_interaction(
    app: AppHandle,
    mode: String,
    has_task: bool,
) -> Result<(), String> {
    let window = main_window(&app)?;
    let mode = IslandMode::from_value(&mode)?;
    mutate_window_state(|state| {
        state.mode = mode;
        state.has_task = has_task;
    });
    window
        .set_ignore_cursor_events(false)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn snap_island_to_edge(app: AppHandle) -> Result<(), String> {
    let window = main_window(&app)?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let stage_size = window.outer_size().map_err(|error| error.to_string())?;
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor is available for edge snapping.".to_string())?;
    let work_area = monitor.work_area();
    let scale = monitor.scale_factor();
    let (island_width, island_height) = read_window_state().size();
    let snapped = snapped_stage_position(
        (position.x, position.y),
        stage_size.width as i32,
        (
            (island_width * scale).round() as i32,
            (island_height * scale).round() as i32,
        ),
        (
            work_area.position.x,
            work_area.position.y,
            work_area.size.width as i32,
            work_area.size.height as i32,
        ),
        (EDGE_SNAP_THRESHOLD * scale).round() as i32,
        (EDGE_SNAP_MARGIN * scale).round() as i32,
    );

    if snapped != (position.x, position.y) {
        window
            .set_position(Position::Physical(PhysicalPosition::new(
                snapped.0, snapped.1,
            )))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn minimize_island(app: AppHandle) -> Result<(), String> {
    hide_island(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn show_ready_island(app: AppHandle) -> Result<(), String> {
    show_island(&app)
}

#[tauri::command]
pub(crate) fn get_launch_at_startup() -> Result<bool, String> {
    Ok(startup_registry_value_exists(STARTUP_REGISTRY_VALUE)?
        || startup_registry_value_exists(LEGACY_STARTUP_REGISTRY_VALUE)?)
}

fn startup_registry_value_exists(value_name: &str) -> Result<bool, String> {
    let mut command = Command::new("reg");
    let status = command
        .creation_flags(CREATE_NO_WINDOW)
        .args(["query", STARTUP_REGISTRY_KEY, "/v", value_name])
        .status()
        .map_err(|error| format!("Failed to query startup registry: {error}"))?;

    Ok(status.success())
}

#[tauri::command]
pub(crate) fn set_launch_at_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        let current_exe = env::current_exe()
            .map_err(|error| format!("Failed to resolve current executable: {error}"))?;
        let startup_value = format!("\"{}\"", current_exe.display());

        let mut command = Command::new("reg");
        let status = command
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "add",
                STARTUP_REGISTRY_KEY,
                "/v",
                STARTUP_REGISTRY_VALUE,
                "/t",
                "REG_SZ",
                "/d",
            ])
            .arg(startup_value)
            .arg("/f")
            .status()
            .map_err(|error| format!("Failed to update startup registry: {error}"))?;
        if !status.success() {
            return Err("Startup registry command failed.".to_string());
        }

        remove_startup_registry_value(LEGACY_STARTUP_REGISTRY_VALUE)?;
        Ok(())
    } else {
        remove_startup_registry_value(STARTUP_REGISTRY_VALUE)?;
        remove_startup_registry_value(LEGACY_STARTUP_REGISTRY_VALUE)
    }
}

#[tauri::command]
pub(crate) fn open_latest_release() -> Result<(), String> {
    Command::new("explorer.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .arg(LATEST_RELEASE_URL)
        .spawn()
        .map_err(|error| format!("Failed to open the Codex Beacon release page: {error}"))?;
    Ok(())
}

fn remove_startup_registry_value(value_name: &str) -> Result<(), String> {
    if !startup_registry_value_exists(value_name)? {
        return Ok(());
    }

    let status = {
        let mut command = Command::new("reg");
        command
            .creation_flags(CREATE_NO_WINDOW)
            .args(["delete", STARTUP_REGISTRY_KEY, "/v", value_name, "/f"])
            .status()
    }
    .map_err(|error| format!("Failed to update startup registry: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("Startup registry command failed.".to_string())
    }
}
fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or_else(|| "Main island window was not found.".to_string())
}

pub(crate) fn show_island(app: &AppHandle) -> Result<(), String> {
    let window = main_window(app)?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

fn hide_island(app: &AppHandle) {
    if let Ok(window) = main_window(app) {
        let _ = window.hide();
    }
}

fn window_state() -> &'static Mutex<IslandWindowState> {
    WINDOW_STATE.get_or_init(|| Mutex::new(IslandWindowState::default()))
}

fn mutate_window_state(update: impl FnOnce(&mut IslandWindowState)) {
    let mut state = window_state().lock().expect("window state poisoned");
    update(&mut state);
}

fn read_window_state() -> IslandWindowState {
    *window_state().lock().expect("window state poisoned")
}

fn apply_stage_geometry(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_size(Size::Logical(LogicalSize::new(
            STAGE_WINDOW_WIDTH,
            STAGE_WINDOW_HEIGHT,
        )))
        .map_err(|error| error.to_string())?;

    let monitor = window
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .or(window
            .current_monitor()
            .map_err(|error| error.to_string())?)
        .ok_or_else(|| "No monitor is available for island positioning.".to_string())?;

    let scale = monitor.scale_factor();
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let physical_width = (STAGE_WINDOW_WIDTH * scale).round() as i32;
    let x = monitor_position.x + ((monitor_size.width as i32 - physical_width) / 2);
    let y = monitor_position.y + (WINDOW_MARGIN_Y * scale).round() as i32;

    window
        .set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|error| error.to_string())
}

fn start_cursor_passthrough_loop(window: WebviewWindow) {
    thread::spawn(move || {
        let mut ignoring_cursor = false;

        loop {
            let should_ignore = !cursor_is_inside_island(&window);
            if should_ignore != ignoring_cursor
                && window.set_ignore_cursor_events(should_ignore).is_ok()
            {
                ignoring_cursor = should_ignore;
            }

            thread::sleep(Duration::from_millis(12));
        }
    });
}

fn cursor_is_inside_island(window: &WebviewWindow) -> bool {
    let hwnd = match window.hwnd() {
        Ok(hwnd) => hwnd,
        Err(_) => return true,
    };
    let mut window_rect = RECT::default();
    let mut cursor = POINT::default();

    if unsafe { GetWindowRect(hwnd, &mut window_rect) }.is_err() {
        return true;
    }
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return true;
    }

    let window_width = (window_rect.right - window_rect.left).max(1) as f64;
    let physical_scale = window_width / STAGE_WINDOW_WIDTH;
    let local_x = (cursor.x - window_rect.left) as f64;
    let local_y = (cursor.y - window_rect.top) as f64;
    let state = read_window_state();
    let (base_width, base_height) = state.size();
    let island_width = base_width * physical_scale;
    let island_height = base_height * physical_scale;
    let island_left = (window_width - island_width) / 2.0;
    let radius = state.corner_radius() * physical_scale;

    point_in_rounded_rect(
        local_x,
        local_y,
        island_left,
        0.0,
        island_width,
        island_height,
        radius,
    )
}

fn point_in_rounded_rect(
    x: f64,
    y: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    radius: f64,
) -> bool {
    let right = left + width;
    let bottom = top + height;

    if x < left || x > right || y < top || y > bottom {
        return false;
    }

    let radius = radius.min(width / 2.0).min(height / 2.0);
    let center_x = if x < left + radius {
        left + radius
    } else if x > right - radius {
        right - radius
    } else {
        x
    };
    let center_y = if y < top + radius {
        top + radius
    } else if y > bottom - radius {
        bottom - radius
    } else {
        y
    };
    let dx = x - center_x;
    let dy = y - center_y;

    (dx * dx) + (dy * dy) <= radius * radius
}

fn snapped_stage_position(
    position: (i32, i32),
    stage_width: i32,
    island_size: (i32, i32),
    work_area: (i32, i32, i32, i32),
    threshold: i32,
    margin: i32,
) -> (i32, i32) {
    let side_padding = (stage_width - island_size.0) / 2;
    (
        snap_axis(
            position.0,
            side_padding,
            island_size.0,
            work_area.0,
            work_area.2,
            threshold,
            margin,
        ),
        snap_axis(
            position.1,
            0,
            island_size.1,
            work_area.1,
            work_area.3,
            threshold,
            margin,
        ),
    )
}

fn snap_axis(
    position: i32,
    visible_offset: i32,
    visible_size: i32,
    work_start: i32,
    work_size: i32,
    threshold: i32,
    margin: i32,
) -> i32 {
    let visible_start = position + visible_offset;
    let visible_end = visible_start + visible_size;
    let target_start = work_start + margin;
    let target_end = work_start + work_size - margin;
    let start_distance = (visible_start - target_start).abs();
    let end_distance = (visible_end - target_end).abs();

    if start_distance <= threshold && start_distance <= end_distance {
        target_start - visible_offset
    } else if end_distance <= threshold {
        target_end - visible_size - visible_offset
    } else {
        position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_island_uses_the_compact_hit_area() {
        let idle = IslandWindowState::default();
        assert_eq!(idle.size(), (320.0, 56.0));
        assert_eq!(idle.corner_radius(), 28.0);
    }

    #[test]
    fn island_stage_snaps_by_the_visible_island_edges() {
        let stage_width = 820;
        let collapsed = (510, 68);
        let work_area = (0, 0, 1920, 1040);

        assert_eq!(
            snapped_stage_position((-150, 200), stage_width, collapsed, work_area, 16, 12),
            (-143, 200)
        );
        assert_eq!(
            snapped_stage_position((1250, 954), stage_width, collapsed, work_area, 16, 12),
            (1243, 960)
        );
        assert_eq!(
            snapped_stage_position((300, 300), stage_width, collapsed, work_area, 16, 12),
            (300, 300)
        );
        assert_eq!(
            snapped_stage_position(
                (-2070, 200),
                stage_width,
                collapsed,
                (-1920, 0, 1920, 1040),
                16,
                12,
            ),
            (-2063, 200)
        );
    }
}

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if startup_registry_value_exists(LEGACY_STARTUP_REGISTRY_VALUE).unwrap_or(false) {
        let _ = set_launch_at_startup(true);
    }
    build_tray(app)?;
    if let Ok(window) = main_window(app.handle()) {
        if let Err(error) = apply_stage_geometry(&window) {
            eprintln!("failed to size and position island window: {error}");
        }
        start_cursor_passthrough_loop(window);
    }
    Ok(())
}

fn build_tray(app: &App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示灵动岛", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "隐藏灵动岛", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    let mut tray = TrayIconBuilder::new()
        .tooltip("Codex Beacon")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                let _ = show_island(app);
            }
            "hide" => hide_island(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_island(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}
