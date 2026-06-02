use std::collections::BTreeMap;

use crate::process::runner::Runner;

pub const ET_FORMULA: &str = "et";
pub const TMUX_FORMULA: &str = "tmux";
pub const MUTAGEN_TAP: &str = "mutagen-io/mutagen";
pub const MUTAGEN_FORMULA: &str = "mutagen-io/mutagen/mutagen";
pub const TAILSCALE_CASK: &str = "tailscale-app";

#[derive(Debug, Clone)]
pub struct Inspector {
    installed: BTreeMap<String, bool>,
}

impl Inspector {
    pub fn from_installed<const N: usize>(items: [(&str, bool); N]) -> Self {
        let mut installed = BTreeMap::new();
        for (name, value) in items {
            installed.insert(name.to_string(), value);
        }
        Self { installed }
    }

    pub fn has(&self, name: &str) -> bool {
        self.installed.get(name).copied().unwrap_or(false)
    }
}

pub fn inspect_installed<R: Runner>(runner: &R) -> Inspector {
    Inspector::from_installed([
        (ET_FORMULA, brew_formula_installed(runner, ET_FORMULA)),
        (TMUX_FORMULA, command_available(runner, "tmux")),
        (
            MUTAGEN_FORMULA,
            brew_formula_installed(runner, MUTAGEN_FORMULA),
        ),
        ("mutagen", command_available(runner, "mutagen")),
        (TAILSCALE_CASK, tailscale_available(runner)),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyPlan {
    pub formulae: Vec<String>,
    pub casks: Vec<String>,
}

struct FormulaRequirement {
    install_name: &'static str,
    detected_names: &'static [&'static str],
}

const FORMULA_REQUIREMENTS: [FormulaRequirement; 3] = [
    FormulaRequirement {
        install_name: ET_FORMULA,
        detected_names: &[ET_FORMULA],
    },
    FormulaRequirement {
        install_name: TMUX_FORMULA,
        detected_names: &[TMUX_FORMULA],
    },
    FormulaRequirement {
        install_name: MUTAGEN_FORMULA,
        detected_names: &["mutagen", MUTAGEN_FORMULA],
    },
];

pub fn required_formulae() -> Vec<String> {
    FORMULA_REQUIREMENTS
        .iter()
        .map(|requirement| requirement.install_name.to_string())
        .collect()
}

pub fn build_dependency_plan(inspector: &Inspector) -> DependencyPlan {
    let formulae = FORMULA_REQUIREMENTS
        .into_iter()
        .filter(|requirement| {
            !requirement
                .detected_names
                .iter()
                .any(|name| inspector.has(name))
        })
        .map(|requirement| requirement.install_name.to_string())
        .collect();

    let casks = [TAILSCALE_CASK]
        .into_iter()
        .filter(|name| !inspector.has(name))
        .map(String::from)
        .collect();

    DependencyPlan { formulae, casks }
}

fn brew_formula_installed<R: Runner>(runner: &R, formula: &str) -> bool {
    runner
        .run(
            "brew",
            &[
                "list".to_string(),
                "--formula".to_string(),
                formula.to_string(),
            ],
        )
        .is_ok_and(|output| output.success)
}

fn command_available<R: Runner>(runner: &R, command: &str) -> bool {
    runner
        .run(
            "sh",
            &[
                "-c".to_string(),
                format!("command -v {command} >/dev/null 2>&1"),
            ],
        )
        .is_ok_and(|output| output.success)
}

fn tailscale_available<R: Runner>(runner: &R) -> bool {
    command_available(runner, "tailscale")
        || runner
            .run(
                "sh",
                &[
                    "-c".to_string(),
                    "test -x /Applications/Tailscale.app/Contents/MacOS/Tailscale".to_string(),
                ],
            )
            .is_ok_and(|output| output.success)
}
