use clap::{Parser, ValueEnum};
use std::collections::BTreeSet;
use std::io::{BufWriter, Read, Write};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::ops::RangeInclusive;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

mod config;
mod domain;
mod parsing;
use domain::*;

use crate::config::Config;

static CONFIG_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::config_dir().expect("could not determine config directory"));
static CURRENT_CONF: LazyLock<PathBuf> =
    LazyLock::new(|| CONFIG_DIR.join("hypr").join("current_conf.lua"));
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
    let config = config::Config::parse_from_config_file();
    match cli.mode {
        Mode::Multi => {
            switch_to_multi(&config, cli.reverse, cli.direction);
        }
        Mode::Laptop => {
            switch_to_laptop();
        }
        Mode::Mirror => {
            switch_to_mirror(&config, resolution_counter);
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

fn switch_to_mirror(config: &Config, resolution_counter: usize) {
    let main_monitor_template = file_to_string(MIRROR_CONF_1);
    let secondary_monitor_template = file_to_string(MIRROR_CONF_2);

    let monitors = parsing::parse_monitors(&parsing::hyprctl_monitors_all(config));
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MonitorType {
    Main,
    Sub,
}

/// all_monitors: Monitors in order from left to right.
/// main_monitors: Which of the monitors are main monitors?
pub fn write_monitor_configs(all_monitors: &mut [(Monitor, MonitorType)], config: &Config) {
    if config.ignore_monitor_type {
        all_monitors
            .iter_mut()
            .for_each(|(_, monitor_type)| *monitor_type = MonitorType::Main);
    }

    // Wenn wir nur einen Monitor haben, dann ist der main :)
    // Auch wenn ich hier gegen Programmierprinzipien verstoße mit dem mut, mir egal :D
    if all_monitors.len() == 1 {
        all_monitors[0].1 = MonitorType::Main;
    }
    let file = std::fs::File::create_new(TMP_CURRENT_CONF.deref()).unwrap();
    let mut settings = BufWriter::new(file);
    let mut offset = 0;
    let mut main_monitor_counter = 0;

    writeln!(settings, "local monitor_list = {{").unwrap();
    for (monitor, _) in all_monitors
        .iter()
        .filter(|(_, monitor_typ)| *monitor_typ == MonitorType::Main)
    {
        writeln!(settings, "\t\"{}\",", monitor.name()).unwrap();
        main_monitor_counter += 1;
    }
    writeln!(settings, "}}\n").unwrap();

    for (monitor, r#type) in all_monitors {
        let mode = monitor.modes().last().unwrap();
        let mut offset_denominator = 1;
        let mut monitor_settings = MonitorSettings::new(monitor, *mode);
        if *r#type == MonitorType::Sub {
            monitor_settings.set_scale("2".to_owned());
            offset_denominator = 2;
        }

        monitor_settings.set_position_offset_x(offset);
        offset += monitor_settings.offset_for_next_monitor() / offset_denominator;
        writeln!(settings, "{monitor_settings}").unwrap();
    }

    let (usual_part_size, ranges) = split_range(1..=10, main_monitor_counter);

    let mut moving_settings = String::new();
    let mut wallpaper_settings = String::new();

    for (monitor_idx, range) in ranges.enumerate() {
        let lua_monitor_idx = monitor_idx + 1;
        {
            use std::fmt::Write;
            writeln!(moving_settings, "-- Moving for monitor {monitor_idx}").unwrap();

            writeln!(wallpaper_settings, "-- Wallpaper for monitor {monitor_idx}").unwrap();
        }

        writeln!(settings, "-- Workspace rules for monitor {monitor_idx}").unwrap();

        let mut first = true;
        let mut first_1 = true;
        for workspace_idx in range {
            //
            //
            {
                let normal_key = workspace_idx % 10;
                let mut alt_key = workspace_idx % usual_part_size;
                if alt_key == 0 {
                    alt_key = usual_part_size;
                }
                if alt_key == 1 {
                    if first_1 {
                        first_1 = false;
                    } else {
                        alt_key += usual_part_size;
                    }
                }
                let alt_key_with_modulo = alt_key % 10;
                use std::fmt::Write;
                writeln!(
                    moving_settings,
                    r#"hl.bind(mainMod .. " + {normal_key}", hl.dsp.focus({{ workspace = "{workspace_idx}" }}))"#
                ).unwrap();
                writeln!(
                    moving_settings,
                    r#"hl.bind(mainMod .. " + SHIFT + {normal_key}", hl.dsp.window.move({{ workspace = "{workspace_idx}" }}))"#
                ).unwrap();
                writeln!(
                    moving_settings,
                    r#"hl.bind(mainMod .. " + ALT + {alt_key_with_modulo}", hl.dsp.focus({{ workspace = "{workspace_idx}" }}))"#
                ).unwrap();

                // Wallpaper
                let wallpaper_idx = ((alt_key - 1) % 5) + 1;
                writeln!(
                    wallpaper_settings,
                    r#"hl.bind(mainMod .. " + {normal_key}", hl.dsp.exec_cmd("hyprctl hyprpaper wallpaper \"" .. monitor_list[{lua_monitor_idx}] .. ", " .. wallpaper[{wallpaper_idx}] .. "\""))"#
                ).unwrap();
                writeln!(
                    wallpaper_settings,
                    r#"hl.bind(mainMod .. " + ALT + {alt_key_with_modulo}", hl.dsp.exec_cmd("hyprctl hyprpaper wallpaper \"" .. monitor_list[{lua_monitor_idx}] .. ", " .. wallpaper[{wallpaper_idx}] .. "\""))"#
                ).unwrap();
                writeln!(
                    wallpaper_settings,
                    r#"hl.bind(mainMod .. " + SHIFT + {normal_key}", hl.dsp.exec_cmd("hyprctl hyprpaper wallpaper \"" .. monitor_list[{lua_monitor_idx}] .. ", " .. wallpaper[{wallpaper_idx}] .. "\""))"#
                ).unwrap();
            }

            writeln!(
                settings,
                "hl.workspace_rule({{
    workspace = {workspace_idx},
    monitor = monitor_list[{lua_monitor_idx}],
    persistent = true,
    default = {first},
}})"
            )
            .unwrap();
            first = false;
        }
    }

    writeln!(settings, "{moving_settings}").unwrap();
    writeln!(settings, "{wallpaper_settings}").unwrap();

    drop(settings);
    std::fs::rename(TMP_CURRENT_CONF.deref(), CURRENT_CONF.deref()).unwrap();
}

fn split_range(
    range: RangeInclusive<usize>,
    parts: usize,
) -> (usize, impl Iterator<Item = RangeInclusive<usize>>) {
    let start = *range.start();
    let end = *range.end();

    let length = end - start + 1;
    let size = length / parts;
    let remainder = length % parts;

    let mut current = start;

    (
        size,
        (1..=parts).map(move |index| {
            let current_size = if index == parts {
                size + remainder
            } else {
                size
            };

            let range_end = current + current_size - 1;
            let range = current..=range_end;
            current = range_end + 1;
            range
        }),
    )
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Direction {
    Left,
    Right,
}

pub fn switch_to_multi(
    config: &Config,
    reverse_main_monitor_order: bool,
    laptop_direction: Direction,
) {
    let monitors = parsing::hyprctl_monitors_all(config);
    let mut monitors: Vec<_> = parsing::parse_monitors(&monitors)
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

    write_monitor_configs(&mut monitors, &config);

    // Hier vielleicht ein command ausführen, damit der Laptop ein Wallpaper hat
    // hyprctl hyprpaper wallpaper "eDP-1, /home/yannick/Pictures/wallpaper/1.png"
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
