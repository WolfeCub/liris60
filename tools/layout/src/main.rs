//! Renders keyboard.toml to site/index.html, drawing each layer in the board's physical shape.

use std::{error::Error, fs, mem};

use maud::{DOCTYPE, Markup, PreEscaped, html};
use rmk_config::{
    KeyboardTomlConfig,
    layout::{Key, LayoutInfo},
    resolved::behavior::{Combo, Fork, Macro, MacroOperation, MorseKey},
};
use rmk_types::keycode::{HidKeyCode, to_ascii};
use serde::{Deserialize, de::value};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
/// Space around the board, in key units.
const MARGIN: f32 = 0.1;

fn main() -> Result<(), Box<dyn Error>> {
    let path = format!("{ROOT}/keyboard.toml");
    let config = KeyboardTomlConfig::new_from_toml_path(&path);
    let mut layout = LayoutInfo::from_compressed_blob(&config.layout()?.blob)?;
    let behavior = config.behavior()?;

    let board = Board {
        name: config.identity()?.name,
        keys: layout.variants.remove(0).keys,
        layers: read_layers(&fs::read_to_string(&path)?, config.keymap()?.keymap),
        morses: behavior.morse.map(|m| m.morses).unwrap_or_default(),
        macros: behavior.macros.map(|m| m.macros).unwrap_or_default(),
        forks: behavior.forks.map(|f| f.forks).unwrap_or_default(),
        combos: behavior.combos.map(|c| c.combos).unwrap_or_default(),
    };

    let html = board.page().into_string();
    fs::create_dir_all(format!("{ROOT}/site"))?;
    fs::write(format!("{ROOT}/site/index.html"), html)?;
    println!("wrote site/index.html");
    Ok(())
}

struct Board {
    name: String,
    /// Physical keys, each placed by its center and rotation.
    keys: Vec<Key>,
    layers: Vec<Layer>,
    morses: Vec<MorseKey>,
    macros: Vec<Macro>,
    forks: Vec<Fork>,
    combos: Vec<Combo>,
}

struct Layer {
    name: String,
    description: String,
    /// Actions by matrix row and column.
    grid: Vec<Vec<String>>,
}

impl Layer {
    fn action(&self, key: &Key) -> &str {
        &self.grid[key.row as usize][key.col as usize]
    }
}

/// What a key shows: its label, a hint underneath, and a CSS class.
#[derive(Default)]
struct Label {
    text: String,
    hint: String,
    class: &'static str,
}

impl Label {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    fn class(mut self, class: &'static str) -> Self {
        self.class = class;
        self
    }
}

impl Board {
    fn page(&self) -> Markup {
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (self.name) " keymap" }
                    style { (PreEscaped(include_str!("style.css"))) }
                }
                body {
                    h1 { (self.name) }
                    @for (n, layer) in self.layers.iter().enumerate() {
                        (self.section(n, layer))
                    }
                    footer { "Generated from " code { "keyboard.toml" } "." }
                }
            }
        }
    }

    fn section(&self, n: usize, layer: &Layer) -> Markup {
        html! {
            section {
                h2 { (n) " " (layer.name) }
                @if !layer.description.is_empty() {
                    p { (layer.description) }
                }
                div.board style=(self.board_size()) {
                    @for key in &self.keys {
                        (self.key(n, layer, key))
                    }
                    @for combo in self.combos.iter().filter(|c| c.layer == Some(n as u8)) {
                        (self.combo(layer, combo))
                    }
                }
            }
        }
    }

    fn key(&self, n: usize, layer: &Layer, key: &Key) -> Markup {
        let label = self.label(layer.action(key));
        let long = label.text.chars().count() > 2;
        // Outline the base layer keys you hold to get here.
        let held = n > 0 && self.held_layer(self.layers[0].action(key)) == Some(n);
        let style = position(key.rect.x, key.rect.y, key.r);
        html! {
            div.key.(label.class).long[long].held[held] style=(style) {
                span { (label.text) }
                @if !label.hint.is_empty() {
                    small { (label.hint) }
                }
            }
        }
    }

    /// A combo's output sits on the bottom edge between the keys it joins.
    fn combo(&self, layer: &Layer, combo: &Combo) -> Markup {
        let find = |action: &String| self.keys.iter().find(|k| layer.action(k) == action);
        let keys: Vec<&Key> = combo.actions.iter().filter_map(find).collect();
        let x = keys.iter().map(|k| k.rect.x).sum::<f32>() / keys.len() as f32;
        let y = keys.iter().map(|k| k.rect.y).sum::<f32>() / keys.len() as f32 + 0.42;
        html! {
            div.combo style=(position(x, y, 0.0)) { (self.label(&combo.output).text) }
        }
    }

    /// CSS variables that size the board to fit every key.
    fn board_size(&self) -> String {
        let width = self.keys.iter().map(|k| k.rect.x).fold(0.0, f32::max) + 0.5 + 2.0 * MARGIN;
        let height = self.keys.iter().map(|k| k.rect.y).fold(0.0, f32::max) + 0.5 + 2.0 * MARGIN;
        format!("--w: {width:.2}; --h: {height:.2}")
    }

    fn label(&self, action: &str) -> Label {
        match parse(self.fork_output(action)) {
            ("_" | "No", _) => Label::default().class("empty"),
            ("MT", args) => Label::new(name(args[0])).hint(name(args[1])),
            ("MO" | "LM", args) => self.layer_label(args[0]),
            ("TG", args) => self.layer_label(args[0]).hint("toggle"),
            ("TD", args) => self.tap_dance_label(&self.morses[index(args[0])]),
            ("Macro", args) => self.macro_label(&self.macros[index(args[0])].operations),
            ("OSM", args) => Label::new(name(args[0])).hint("one-shot"),
            ("WM", args) => Label::new(format!("{}+{}", name(args[1]), name(args[0]))),
            ("SHIFTED", args) => Label::new(shifted(args[0])),
            (keycode, _) => Label::new(name(keycode)),
        }
    }

    fn layer_label(&self, layer: &str) -> Label {
        Label::new(&self.layers[index(layer)].name).class("layer")
    }

    /// Shows the tap action (or the hold, without one) with the double tap as the hint.
    fn tap_dance_label(&self, morse: &MorseKey) -> Label {
        let (tap, hold) = (morse.tap.as_deref(), morse.hold.as_deref());
        let mut label = self.label(tap.or(hold).expect("tap dance needs a tap or hold"));
        if hold.is_some_and(|hold| self.held_layer(hold).is_some()) {
            label = label.class("layer");
        }
        if let Some(double_tap) = morse.double_tap.as_deref() {
            let double_tap = self.label(double_tap);
            // Double tapping Raise toggles Raise, which reads better as "2× toggle".
            let what = if double_tap.text == label.text {
                double_tap.hint
            } else {
                double_tap.text
            };
            label = label.hint(format!("2× {what}"));
        }
        label
    }

    /// Shows the key the macro presses.
    fn macro_label(&self, operations: &[MacroOperation]) -> Label {
        let pressed = operations.iter().find_map(|op| match op {
            MacroOperation::Down { keycode } => Some(name(keycode)),
            _ => None,
        });
        Label::new(pressed.unwrap_or_else(|| "Macro".into())).hint("macro")
    }

    /// The layer an action switches on while it's held.
    fn held_layer(&self, action: &str) -> Option<usize> {
        match parse(action) {
            ("MO" | "LM", args) => Some(index(args[0])),
            ("TD", args) => {
                let hold = self.morses[index(args[0])].hold.as_deref();
                hold.and_then(|hold| self.held_layer(hold))
            }
            _ => None,
        }
    }

    /// What a fork's placeholder keycode actually types.
    fn fork_output<'a>(&'a self, action: &'a str) -> &'a str {
        let fork = self.forks.iter().find(|fork| fork.trigger == action);
        fork.map_or(action, |fork| &fork.positive_output)
    }
}

/// Reads each layer's name and description from the comment above its `[[keymap.layer]]`:
/// a `# Layer N: Name` line, then any description lines.
fn read_layers(toml: &str, grids: Vec<Vec<Vec<String>>>) -> Vec<Layer> {
    let mut comments = Vec::new();
    let mut comment = Vec::new();
    for line in toml.lines() {
        if let Some(text) = line.strip_prefix('#') {
            comment.push(text.trim());
        } else if line == "[[keymap.layer]]" {
            comments.push(mem::take(&mut comment));
        } else {
            comment.clear();
        }
    }

    grids
        .into_iter()
        .zip(comments)
        .map(|(grid, comment)| {
            let heading = comment.first().copied().unwrap_or_default();
            Layer {
                name: heading
                    .split_once(": ")
                    .map_or(heading, |(_, name)| name)
                    .to_string(),
                description: comment.get(1..).unwrap_or_default().join(" "),
                grid,
            }
        })
        .collect()
}

/// Splits `MT(Escape, LCtrl)` into `("MT", ["Escape", "LCtrl"])`. A plain keycode has no arguments.
fn parse(action: &str) -> (&str, Vec<&str>) {
    match action.strip_suffix(')').and_then(|a| a.split_once('(')) {
        Some((function, args)) => (function, args.split(',').map(str::trim).collect()),
        None => (action, Vec::new()),
    }
}

fn index(n: &str) -> usize {
    n.parse().expect("expected a number")
}

/// CSS variables that place a key: its center and rotation.
fn position(x: f32, y: f32, degrees: f32) -> String {
    format!(
        "--x: {:.2}; --y: {:.2}; --r: {degrees}deg",
        x + MARGIN,
        y + MARGIN
    )
}

fn name(keycode: &str) -> String {
    if let Some(symbol) = symbol(keycode, false) {
        return symbol.to_string();
    }
    let name = match keycode {
        "LShift" | "RShift" => "Shift",
        "LCtrl" | "RCtrl" => "Ctrl",
        "LAlt" | "RAlt" => "Alt",
        "LGui" | "RGui" => "Gui",
        "Backspace" => "⌫",
        "Escape" => "Esc",
        "Delete" => "Del",
        "Insert" => "Ins",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "PrintScreen" => "PrtSc",
        "Left" => "←",
        "Right" => "→",
        "Up" => "↑",
        "Down" => "↓",
        "CapsWordToggle" => "Caps Word",
        "AudioVolUp" => "Vol+",
        "AudioVolDown" => "Vol−",
        "AudioMute" => "Mute",
        "MediaPlayPause" => "Play",
        "MediaPrevTrack" => "Prev",
        "MediaNextTrack" => "Next",
        "MouseUp" => "M↑",
        "MouseDown" => "M↓",
        "MouseLeft" => "M←",
        "MouseRight" => "M→",
        "MouseBtn1" => "Click",
        "MouseBtn2" => "R Click",
        "MouseAccel1" => "Accel 1",
        "MouseAccel2" => "Accel 2",
        // Keypad digits.
        _ => keycode
            .strip_prefix("Kp")
            .filter(|d| d.len() == 1)
            .unwrap_or(keycode),
    };
    name.to_string()
}

fn shifted(keycode: &str) -> String {
    symbol(keycode, true).map_or_else(|| format!("⇧{}", name(keycode)), String::from)
}

/// The digit or symbol a keycode types on a US layout.
fn symbol(keycode: &str, shifted: bool) -> Option<char> {
    let code =
        HidKeyCode::deserialize(value::StrDeserializer::<value::Error>::new(keycode)).ok()?;
    let c = to_ascii(code, shifted) as char;
    (c.is_ascii_digit() || c.is_ascii_punctuation()).then_some(c)
}
