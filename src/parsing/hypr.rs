use std::{borrow::Cow, process::Command};

use crate::config::Config;

pub fn hyprctl_monitors_all(config: &'_ Config) -> Cow<'_, str> {
    match &config.monitors_all_replacement {
        Some(replacement) => Cow::Borrowed(replacement.as_str()),
        None => {
            let stdout = Command::new("hyprctl")
                .args(["monitors", "all"])
                .output()
                .unwrap()
                .stdout;
            Cow::Owned(String::from_utf8(stdout).unwrap())
        }
    }
}
