use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::domain::Monitor;

/// Speichert pro Monitor-Kombination, welche Permutation zuletzt gewählt wurde.
/// Key: sortierte Namen der externen Monitore, z.B. "DP-1,DP-2,HDMI-A-1"
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct OrderState {
    #[serde(default)]
    orders: BTreeMap<String, usize>,
}

fn state_path() -> PathBuf {
    dirs::state_dir()
        .expect("could not determine state directory")
        .join("hypr_switch")
        .join("orders.toml")
}

fn load_state() -> OrderState {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_state(state: &OrderState) {
    let path = state_path();
    std::fs::create_dir_all(path.parent().unwrap()).expect("Could not create state directory");
    let content = toml::to_string_pretty(state).expect("Could not serialize order state");
    std::fs::write(path, content).expect("Could not write order state");
}

fn key(monitors: &[Monitor]) -> String {
    let mut names: Vec<_> = monitors.iter().map(|m| m.name()).collect();
    names.sort();
    names.join(",")
}

/// Alle Permutationen von 0..n in "plain changes"-Reihenfolge (Steinhaus-Johnson-Trotter):
/// Zwei aufeinanderfolgende Permutationen unterscheiden sich nur durch einen Tausch
/// zweier benachbarter Elemente (auch vom letzten zurück zum ersten).
/// Beim Durchschalten ändert sich also immer nur ein Monitorpaar.
fn plain_changes(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    for (i, perm) in plain_changes(n - 1).into_iter().enumerate() {
        let positions: Box<dyn Iterator<Item = usize>> = if i % 2 == 0 {
            Box::new((0..n).rev())
        } else {
            Box::new(0..n)
        };
        for pos in positions {
            let mut next = perm.clone();
            next.insert(pos, n - 1);
            out.push(next);
        }
    }
    out
}

/// Sortiert die Monitore deterministisch (nach Namen) und wendet dann die
/// gespeicherte Permutation an. `step` verschiebt die gespeicherte Permutation
/// vorher (z.B. +1 für "next", -1 für "prev") und speichert das Ergebnis.
pub fn apply_stored_order(mut monitors: Vec<Monitor>, step: isize) -> Vec<Monitor> {
    monitors.sort_by(|a, b| a.name().cmp(b.name()));

    let perms = plain_changes(monitors.len());
    let mut state = load_state();
    let key = key(&monitors);
    let current = state.orders.get(&key).copied().unwrap_or(0);
    let index = (current as isize + step).rem_euclid(perms.len() as isize) as usize;

    if step != 0 {
        state.orders.insert(key, index);
        save_state(&state);
    }

    let mut slots: Vec<Option<Monitor>> = monitors.into_iter().map(Some).collect();
    let ordered: Vec<Monitor> = perms[index]
        .iter()
        .map(|&i| slots[i].take().unwrap())
        .collect();

    let names: Vec<_> = ordered.iter().map(|m| m.name()).collect();
    let summary = format!("Anordnung {}/{}: {}", index + 1, perms.len(), names.join(" | "));
    println!("{summary}");
    // Feedback beim Durchtappen per Keybind; Fehler (kein notify-send) sind egal
    let _ = std::process::Command::new("notify-send")
        .args(["-t", "2000", "hypr_switch", &summary])
        .status();

    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn plain_changes_are_complete_and_adjacent() {
        for n in 1..=5 {
            let perms = plain_changes(n);
            let expected: usize = (1..=n).product();
            assert_eq!(perms.len(), expected);
            assert_eq!(perms.iter().collect::<BTreeSet<_>>().len(), expected);

            if n < 2 {
                continue;
            }
            for i in 0..perms.len() {
                let a = &perms[i];
                let b = &perms[(i + 1) % perms.len()];
                let diff: Vec<_> = (0..n).filter(|&j| a[j] != b[j]).collect();
                assert_eq!(diff.len(), 2, "{a:?} -> {b:?}");
                assert_eq!(diff[1] - diff[0], 1, "{a:?} -> {b:?}");
            }
        }
    }
}
