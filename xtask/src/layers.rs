//! Dependency layering rules for DitherCraft.
//!
//! Enforces:
//! - Layer 1 (`dither-core`): Pure DSP, no UI dependencies.
//! - Layer 2 (`dithercraft-cli`): Headless CLI & batch processing.
//! - Layer 3 (`dithercraft`): Native desktop UI (egui/eframe).
//! - `xtask`: Tooling exempt.

use serde_json::Value;

/// Where a workspace crate sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Layer(u8),
    Exempt,
}

impl Class {
    fn layer(self) -> Option<u8> {
        match self {
            Class::Layer(l) => Some(l),
            Class::Exempt => None,
        }
    }
}

pub const TABLE: &[(&str, Class)] =
    &[("dither-core", Class::Layer(1)), ("dithercraft-cli", Class::Layer(2)), ("dithercraft", Class::Layer(3)), ("xtask", Class::Exempt)];

/// External crates that constitute a UI toolkit / windowing dependency.
pub const UI_CRATES: &[&str] = &["egui", "eframe", "winit", "rfd"];

/// First layer allowed to use UI crates.
pub const UI_MIN_LAYER: u8 = 3;

pub fn short_name(pkg: &str) -> &str {
    pkg.strip_prefix("dithercraft-").unwrap_or(pkg)
}

pub fn classify(pkg: &str) -> Option<Class> {
    let s = short_name(pkg);
    TABLE.iter().find(|(n, _)| *n == s || *n == pkg).map(|(_, c)| *c)
}

pub fn describe(class: Option<Class>) -> String {
    match class {
        Some(Class::Layer(l)) => format!("Layer {l}"),
        Some(Class::Exempt) => "Exempt".into(),
        None => "Unregistered".into(),
    }
}

fn is_ui_crate(name: &str) -> bool {
    UI_CRATES.iter().any(|p| match p.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *p,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
    pub workspace: bool,
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub deps: Vec<Dep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Unregistered { krate: String },
    Upward { krate: String, dep: String, from: u8, to: u8, kind: DepKind },
    UiLeak { krate: String, dep: String, layer: u8 },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unregistered { krate } => write!(f, "{krate}: not registered in layers::TABLE"),
            Self::Upward { krate, dep, from, to, kind } => {
                write!(f, "{krate} (L{from}) depends on {dep} (L{to}) [{kind:?}]")
            }
            Self::UiLeak { krate, dep, layer } => {
                write!(f, "{krate} (L{layer}) depends on UI crate `{dep}` (allowed only in L{UI_MIN_LAYER}+)")
            }
        }
    }
}

pub fn check(crates: &[Crate]) -> Vec<Violation> {
    let mut out = Vec::new();
    for c in crates {
        let class = match classify(&c.name) {
            Some(c) => c,
            None => {
                out.push(Violation::Unregistered { krate: c.name.clone() });
                continue;
            }
        };
        let from_layer = match class.layer() {
            Some(l) => l,
            None => continue,
        };

        for d in &c.deps {
            if !d.workspace && is_ui_crate(&d.name) && from_layer < UI_MIN_LAYER && d.kind != DepKind::Dev {
                out.push(Violation::UiLeak { krate: c.name.clone(), dep: d.name.clone(), layer: from_layer });
            }

            if d.workspace {
                let to_class = match classify(&d.name) {
                    Some(c) => c,
                    None => continue,
                };
                let to_layer = match to_class.layer() {
                    Some(l) => l,
                    None => continue,
                };

                if to_layer > from_layer {
                    out.push(Violation::Upward { krate: c.name.clone(), dep: d.name.clone(), from: from_layer, to: to_layer, kind: d.kind });
                }
            }
        }
    }
    out
}

pub fn from_metadata(meta: &Value) -> Result<Vec<Crate>, String> {
    let packages = meta["packages"].as_array().ok_or("cargo metadata: missing packages array")?;
    let workspace_members: Vec<String> = meta["workspace_members"]
        .as_array()
        .ok_or("cargo metadata: missing workspace_members")?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.split_whitespace().next().unwrap_or(s).to_string()))
        .collect();

    let mut crates = Vec::new();
    for p in packages {
        let name = p["name"].as_str().unwrap_or("").to_string();
        if !workspace_members.contains(&name) {
            continue;
        }

        let mut deps = Vec::new();
        if let Some(raw_deps) = p["dependencies"].as_array() {
            for d in raw_deps {
                let d_name = d["name"].as_str().unwrap_or("").to_string();
                let kind = match d["kind"].as_str() {
                    Some("dev") => DepKind::Dev,
                    Some("build") => DepKind::Build,
                    _ => DepKind::Normal,
                };
                let ws = workspace_members.contains(&d_name);
                deps.push(Dep { name: d_name, kind, workspace: ws });
            }
        }
        crates.push(Crate { name, deps });
    }
    Ok(crates)
}
