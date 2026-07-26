use clap::{Parser, ValueEnum};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Read, Write};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::process::*;
use std::sync::LazyLock;

mod domain;
use domain::*;

static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::config_dir()
        .expect("could not determine config directory")
        .join("hypr")
});
static CURRENT_CONF: LazyLock<PathBuf> = LazyLock::new(|| CONFIG_DIR.join("current.conf"));
static TMP_CURRENT_CONF: LazyLock<PathBuf> =
    LazyLock::new(|| CURRENT_CONF.with_added_extension("tmp"));
const LAPTOP_CONF: &str = "/home/yannick/.config/hypr/laptop_hypr.conf";
const MIRROR_CONF_1: &str = "/home/yannick/.config/hypr/mirror_hypr_1.conf";
const MIRROR_CONF_2: &str = "/home/yannick/.config/hypr/mirror_hypr_2.conf";

/// Simple CLI to choose between "arbeit" or "laptop"
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(value_enum)]
    mode: Mode,

    #[arg(long, short)]
    aufloesung: Option<NonZeroUsize>,

    #[arg(long, short)]
    reverse: bool,

    #[arg(long, value_enum, default_value_t = Direction::Right)]
    direction: Direction,
}

#[derive(ValueEnum, Clone)]
enum Mode {
    Multi,
    Laptop,
    Mirror,
}

fn main() {
    let cli = Cli::parse();
    let resolution_counter = cli.aufloesung.map(|r| r.get()).unwrap_or_default();
    match cli.mode {
        Mode::Multi => {
            switch_to_multi(cli.reverse, cli.direction);
        }
        Mode::Laptop => {
            switch_to_laptop();
        }
        Mode::Mirror => {
            switch_to_mirror(resolution_counter);
        }
    }
}

fn switch_to_laptop() {
    let path = std::fs::read_link(LAPTOP_CONF).unwrap();
    let mut reader = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    let _success = reader.read_to_end(&mut buf).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(TMP_CURRENT_CONF.deref())
        .unwrap();
    file.write_all(&buf).unwrap();
    std::fs::rename(TMP_CURRENT_CONF.deref(), CURRENT_CONF.deref()).unwrap();
}

fn file_to_string<P>(path: P) -> String
where
    P: AsRef<Path>,
{
    let path = std::fs::read_link(path).unwrap();
    let mut reader = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    let _success = reader.read_to_end(&mut buf).unwrap();

    String::from_utf8(buf).unwrap()
}

fn switch_to_mirror(resolution_counter: usize) {
    let main_monitor_template = file_to_string(MIRROR_CONF_1);
    let secondary_monitor_template = file_to_string(MIRROR_CONF_2);

    let monitors = parse_monitors(&hyprctl_monitors_all());
    let best = best_monitor_mode(&monitors, resolution_counter);

    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(CURRENT_CONF.deref())
        .unwrap();
    let mut buf_writer = BufWriter::with_capacity(128, file);

    let main_monitor = monitors
        .iter()
        .find(|m| m.name() == "eDP-1")
        .unwrap_or(&monitors[0]);

    let main_monitor_writing =
        main_monitor_template.replace("<<monitor_main>>", main_monitor.name());
    writeln!(buf_writer, "{main_monitor_writing}").unwrap();

    for monitor in monitors.iter() {
        // TODO: Add monitor stuff
        let mut monitor_settings = MonitorSettings::new(monitor, best);
        if monitor.name() != main_monitor.name() {
            monitor_settings.mirror(main_monitor);
        }
        write!(buf_writer, "{monitor_settings}").unwrap();
        let to_write = secondary_monitor_template.replace("<<monitor>>", monitor.name());
        writeln!(buf_writer, "{to_write}").unwrap();
    }
}

fn hyprctl_monitors_all() -> String {
    let stdout = Command::new("hyprctl")
        .args(["monitors", "all"])
        .output()
        .unwrap()
        .stdout;
    String::from_utf8(stdout).unwrap()
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MonitorType {
    Main,
    Sub,
}

/// all_monitors: Monitors in order from left to right.
/// main_monitors: Which of the monitors are main monitors?
pub fn write_monitor_configs(all_monitors: &[(Monitor, MonitorType)]) {
    let file = std::fs::File::create_new(TMP_CURRENT_CONF.deref()).unwrap();
    let mut settings = BufWriter::new(file);
    let mut offset = 0;
    let mut main_monitor_counter = 0;
    for (monitor, r#type) in all_monitors {
        let mode = monitor.modes().last().unwrap();
        let mut offset_denominator = 1;
        let mut monitor_settings = MonitorSettings::new(monitor, *mode);
        if *r#type == MonitorType::Main {
            writeln!(
                settings,
                "$monitor{main_monitor_counter}={}",
                monitor.name()
            )
            .unwrap();
            main_monitor_counter += 1;
        } else {
            monitor_settings.set_scale("2".to_owned());
            offset_denominator = 2;
        }

        monitor_settings.set_position_offset_x(offset);
        offset += monitor_settings.offset_for_next_monitor() / offset_denominator;
        writeln!(settings, "{monitor_settings}").unwrap();
    }

    // Write workspaces
    for monitor_id in 0..main_monitor_counter {
        let workspace_start = 1 + monitor_id * 5;
        writeln!(
            settings,
            "workspace={workspace_start},monitor:$monitor{monitor_id},persistent:true,default:true"
        )
        .unwrap();
        for workspace in workspace_start + 1..workspace_start + 5 {
            writeln!(
                settings,
                "workspace={workspace},monitor:$monitor{monitor_id},persistent:true"
            )
            .unwrap();
        }
    }

    let mut workspace = 1;

    for monitor_id in 0..main_monitor_counter {
        for i in 1..=5 {
            let keyboard_key = workspace % 10;
            writeln!(
                settings,
                "bind=$mainMod, {keyboard_key}, exec, hyprctl hyprpaper wallpaper \"$monitor{monitor_id}, $wall{i}\""
            ).unwrap();

            writeln!(
                settings,
                "bind=$mainMod SHIFT, {keyboard_key}, exec, hyprctl hyprpaper wallpaper \"$monitor{monitor_id}, $wall{i}\""
            ).unwrap();

            writeln!(settings, "bind=$mainMod ALT, {i}, workspace, {workspace}").unwrap();

            writeln!(
                settings,
                "bind=$mainMod ALT, {i}, exec, hyprctl hyprpaper wallpaper \"$monitor{monitor_id}, $wall{i}\""
            )
            .unwrap();

            workspace += 1;
        }
    }

    drop(settings);
    std::fs::rename(TMP_CURRENT_CONF.deref(), CURRENT_CONF.deref()).unwrap();
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Direction {
    Left,
    Right,
}

pub fn switch_to_multi(reverse_main_monitor_order: bool, laptop_direction: Direction) {
    let mut monitors: Vec<_> = parse_monitors(&hyprctl_monitors_all())
        .into_iter()
        .map(|monitor| {
            let r#type = if monitor.is_laptop() {
                MonitorType::Sub
            } else {
                MonitorType::Main
            };
            (monitor, r#type)
        })
        .collect();

    // Now correct ordering of monitors:

    // First remove laptop
    let laptop_index = monitors
        .iter()
        .position(|(_, r#type)| *r#type == MonitorType::Sub)
        .unwrap();
    let laptop = monitors.remove(laptop_index);

    if reverse_main_monitor_order {
        monitors.reverse();
    }

    match laptop_direction {
        Direction::Left => {
            monitors.insert(0, laptop);
        }
        Direction::Right => {
            monitors.push(laptop);
        }
    }

    write_monitor_configs(&monitors);

    // Hier vielleicht ein command ausführen, damit der Laptop ein Wallpaper hat
    // hyprctl hyprpaper wallpaper "eDP-1, /home/yannick/Pictures/wallpaper/1.png"
}

fn parse_monitors(to_parse: &str) -> Vec<Monitor> {
    let monitors = to_parse.split("Monitor ").skip(1);

    let monitors: Vec<_> = monitors
        .map(|monitor| {
            let (monitor_name, rest) = monitor.split_once(" ").unwrap();
            let mut vrr = false;
            let mut modes: BTreeSet<MonitorMode> = BTreeSet::new();
            let mut speed_map = BTreeMap::new();
            for line in rest.lines() {
                let line = line.trim_start();
                if let Some(vrr_str) = line.strip_prefix("vrr: ") {
                    vrr = vrr_str.parse().unwrap_or(false);
                    continue;
                }
                if let Some(available_modes) = line.strip_prefix("availableModes: ") {
                    for mode in available_modes.split_ascii_whitespace() {
                        let (mode, speed) = monitor_mode_and_speed(mode);
                        modes.insert(mode);

                        speed_map
                            .entry(mode)
                            .or_insert_with(BTreeSet::new)
                            .insert(speed);
                    }
                }
            }
            let name = monitor_name.to_owned();
            Monitor::new(name, vrr, modes, speed_map)
        })
        .collect();
    monitors
}

fn monitor_mode_and_speed(s: &str) -> (MonitorMode, MonitorSpeed) {
    let (x, rest) = s.split_once('x').unwrap();
    let (y, rest) = rest.split_once('@').unwrap();

    let (speed_high, rest) = rest.split_once('.').unwrap();
    let speed_low = rest.strip_suffix("Hz").unwrap();

    (
        MonitorMode {
            x: x.parse().unwrap(),
            y: y.parse().unwrap(),
        },
        MonitorSpeed {
            speed_high: speed_high.parse().unwrap(),
            speed_low: speed_low.parse().unwrap(),
        },
    )
}

fn best_monitor_mode(monitors: &[Monitor], nth: usize) -> MonitorMode {
    let set = if monitors.len() > 1 {
        let (first_2, rest) = monitors.split_at(2);

        let mut set: BTreeSet<_> = first_2[0]
            .modes()
            .intersection(first_2[1].modes())
            .cloned()
            .collect();

        for other in rest {
            set = set.intersection(other.modes()).cloned().collect();
        }
        set
    } else {
        // ich ignoriere den Fall das 0 Monitore da sind, wir sollten immer mindestens 1 monitor haben
        // und da wir returned hätten, wenn wir genau 1 monitor haben,
        // gehe ich von mindestens 2 aus
        monitors[0].modes().clone()
    };

    dbg!(&set);

    set.into_iter().rev().nth(nth).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_monitors() {
        let monitor_str = "Monitor eDP-1 (ID 0):
	1920x1200@120.00000 at 3840x0
	description: China Star Optoelectronics Technology Co. Ltd MNE007ZA3-2
	make: China Star Optoelectronics Technology Co., Ltd
	model: MNE007ZA3-2
	physical size (mm): 300x190
	serial:
	active workspace: 1 (1)
	special workspace: 0 ()
	reserved: 0 35 0 0
	scale: 1.00
	transform: 0
	focused: yes
	dpmsStatus: 1
	vrr: true
	solitary: 0
	solitaryBlockedBy: windowed mode,missing candidate
	activelyTearing: false
	tearingBlockedBy: next frame is not torn,user settings,missing candidate
	directScanoutTo: 0
	directScanoutBlockedBy: user settings,missing candidate
	disabled: false
	currentFormat: XRGB2101010
	mirrorOf: none
	availableModes: 2880x1800@120.00Hz 2880x1800@60.00Hz 2880x1800@48.00Hz 1920x1200@120.00Hz 1920x1080@120.00Hz 1600x1200@120.00Hz 1680x1050@120.00Hz 1280x1024@120.00Hz 1440x900@120.00Hz 1280x800@120.00Hz 1280x720@120.00Hz 1024x768@120.00Hz 800x600@120.00Hz 640x480@120.00Hz

    Monitor eDP-2 (ID 0):
	1920x1200@120.00000 at 3840x0
	description: China Star Optoelectronics Technology Co. Ltd MNE007ZA3-2
	make: China Star Optoelectronics Technology Co., Ltd
	model: MNE007ZA3-2
	physical size (mm): 300x190
	serial:
	active workspace: 1 (1)
	special workspace: 0 ()
	reserved: 0 35 0 0
	scale: 1.00
	transform: 0
	focused: yes
	dpmsStatus: 1
	vrr: true
	solitary: 0
	solitaryBlockedBy: windowed mode,missing candidate
	activelyTearing: false
	tearingBlockedBy: next frame is not torn,user settings,missing candidate
	directScanoutTo: 0
	directScanoutBlockedBy: user settings,missing candidate
	disabled: false
	currentFormat: XRGB2101010
	mirrorOf: none
	availableModes: 1920x1200@120.00Hz 1920x1080@120.00Hz 1600x1200@120.00Hz 1680x1050@120.00Hz 1280x1024@120.00Hz 1440x900@120.00Hz 1280x800@120.00Hz 1280x720@120.00Hz 1024x768@120.00Hz 800x600@120.00Hz 640x480@120.00Hz";

        let monitors = parse_monitors(monitor_str);
        dbg!(&monitors);
        let best = best_monitor_mode(&monitors, 0);
        println!("{best}");
    }
}
