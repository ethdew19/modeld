//! modeld GUI: edit a Glorious Model D's settings and apply them to the mouse.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, ComboBox, RichText, Slider, Ui};
use modeld_core::{
    ButtonAction, Buttons, Config, Direction, LiftOff, LightingMode, MAX_DPI, MAX_SPEED, MIN_DPI,
    ModelD, MouseButton, PHYSICAL_BUTTONS, PollingRate, Rgb, key_name,
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("modeld")
            .with_inner_size([620.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("modeld", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Performance,
    Lighting,
    Buttons,
}

struct App {
    mouse: Option<ModelD>,
    /// What's on the mouse (last read / written).
    saved: Option<(Config, Buttons)>,
    /// What's being edited.
    config: Option<Config>,
    buttons: Option<Buttons>,
    /// Debounce can't be read back, so it's sent on its own.
    debounce: u8,
    tab: Tab,
    status: Result<String, String>,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            mouse: None,
            saved: None,
            config: None,
            buttons: None,
            debounce: 10,
            tab: Tab::Performance,
            status: Ok(String::new()),
        };
        app.reload();
        app
    }

    /// (Re)connects if needed and reads everything from the mouse.
    fn reload(&mut self) {
        let result = (|| {
            if self.mouse.is_none() {
                self.mouse = Some(ModelD::open()?);
            }
            let mouse = self.mouse.as_ref().unwrap();
            Ok::<_, modeld_core::Error>((mouse.read_config()?, mouse.read_buttons()?))
        })();
        match result {
            Ok((config, buttons)) => {
                self.config = Some(config.clone());
                self.buttons = Some(buttons.clone());
                self.saved = Some((config, buttons));
                self.status = Ok("read settings from the mouse".into());
            }
            Err(e) => {
                // Drop the handle so the next attempt reconnects (e.g. after replugging).
                self.mouse = None;
                self.status = Err(e.to_string());
            }
        }
    }

    fn apply(&mut self) {
        let (Some(mouse), Some(config), Some(buttons)) = (&self.mouse, &self.config, &self.buttons)
        else {
            return;
        };
        let result = (|| {
            let (saved_config, saved_buttons) = self.saved.as_ref().unwrap();
            if config != saved_config {
                mouse.write_config(config)?;
            }
            if buttons != saved_buttons {
                mouse.write_buttons(buttons)?;
            }
            Ok::<_, modeld_core::Error>(())
        })();
        match result {
            Ok(()) => {
                self.reload();
                self.status = Ok("applied".into());
            }
            Err(e) => self.status = Err(e.to_string()),
        }
    }

    fn dirty(&self) -> bool {
        match (&self.saved, &self.config, &self.buttons) {
            (Some((c, b)), Some(config), Some(buttons)) => c != config || b != buttons,
            _ => false,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Glorious Model D");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let dirty = self.dirty();
                    if ui.add_enabled(dirty, egui::Button::new("Apply")).clicked() {
                        self.apply();
                    }
                    if ui
                        .button("Reload")
                        .on_hover_text("Discard edits and read the mouse")
                        .clicked()
                    {
                        self.reload();
                    }
                    if dirty {
                        ui.label(RichText::new("unsaved changes").italics());
                    }
                });
            });
            ui.horizontal(|ui| {
                for (tab, name) in [
                    (Tab::Performance, "Performance"),
                    (Tab::Lighting, "Lighting"),
                    (Tab::Buttons, "Buttons"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, name);
                }
            });
            ui.add_space(2.0);
        });

        egui::Panel::bottom("status").show(ui, |ui| match &self.status {
            Ok(msg) => {
                ui.label(msg);
            }
            Err(msg) => {
                ui.colored_label(Color32::LIGHT_RED, msg);
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let (Some(config), Some(buttons)) = (&mut self.config, &mut self.buttons) else {
                ui.label("Mouse not connected. Plug it in and press Reload.");
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Performance => {
                    performance(ui, config);
                    ui.separator();
                    if let Some(msg) = debounce(ui, &mut self.debounce, self.mouse.as_ref()) {
                        self.status = msg;
                    }
                }
                Tab::Lighting => lighting(ui, config),
                Tab::Buttons => buttons_tab(ui, buttons),
            });
        });
    }
}

fn performance(ui: &mut Ui, c: &mut Config) {
    egui::Grid::new("perf")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Polling rate");
            ComboBox::from_id_salt("poll")
                .selected_text(format!("{} Hz", c.polling_rate.hz()))
                .show_ui(ui, |ui| {
                    for rate in PollingRate::ALL {
                        ui.selectable_value(&mut c.polling_rate, rate, format!("{} Hz", rate.hz()));
                    }
                });
            ui.end_row();

            ui.label("Lift-off distance");
            ui.horizontal(|ui| {
                ui.radio_value(&mut c.lift_off, LiftOff::Mm2, "2 mm");
                ui.radio_value(&mut c.lift_off, LiftOff::Mm3, "3 mm");
            });
            ui.end_row();

            ui.label("XY independent");
            if ui
                .checkbox(&mut c.xy_independent, "separate X and Y DPI")
                .changed()
                && !c.xy_independent
            {
                for s in &mut c.stages {
                    s.y = s.x;
                }
            }
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(RichText::new("DPI stages").strong());
    let active = c.effective_active_stage();
    let xy = c.xy_independent;
    egui::Grid::new("stages")
        .num_columns(5)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("active");
            ui.label("stage");
            ui.label(if xy { "X DPI" } else { "DPI" });
            ui.label(if xy { "Y DPI" } else { "" });
            ui.label("light");
            ui.end_row();
            for (i, s) in c.stages.iter_mut().enumerate() {
                let n = i as u8 + 1;
                ui.add_enabled_ui(s.enabled, |ui| {
                    if ui.radio(active == n, "").clicked() {
                        c.active_stage = n;
                    }
                });
                ui.checkbox(&mut s.enabled, format!("{n}"));
                ui.add_enabled_ui(s.enabled, |ui| {
                    let x = ui.add(
                        Slider::new(&mut s.x, MIN_DPI..=MAX_DPI)
                            .step_by(100.0)
                            .logarithmic(true),
                    );
                    if x.changed() && !xy {
                        s.y = s.x;
                    }
                });
                if xy {
                    ui.add_enabled(
                        s.enabled,
                        Slider::new(&mut s.y, MIN_DPI..=MAX_DPI)
                            .step_by(100.0)
                            .logarithmic(true),
                    );
                } else {
                    ui.label("");
                }
                color_button(ui, &mut s.color);
                ui.end_row();
            }
        });
}

/// Debounce is write-only, so it has its own button. Returns a status message when sent.
fn debounce(ui: &mut Ui, ms: &mut u8, mouse: Option<&ModelD>) -> Option<Result<String, String>> {
    let mut status = None;
    ui.horizontal(|ui| {
        ui.label("Debounce time");
        ui.add(Slider::new(ms, 4..=16).step_by(2.0).suffix(" ms"));
        if ui
            .add_enabled(mouse.is_some(), egui::Button::new("Send"))
            .clicked()
        {
            status = Some(match mouse.unwrap().set_debounce(*ms) {
                Ok(()) => Ok(format!("debounce set to {ms} ms")),
                Err(e) => Err(e.to_string()),
            });
        }
    });
    ui.label(
        RichText::new("The mouse can't report its debounce time, so this isn't read back.").weak(),
    );
    status
}

fn lighting(ui: &mut Ui, c: &mut Config) {
    let l = &mut c.lighting;
    ui.checkbox(&mut l.enabled, "Lighting on")
        .on_hover_text("Master switch: off keeps the mode and colours. The DPI light still works.");
    ui.add_space(4.0);
    ComboBox::from_label("Mode")
        .selected_text(mode_label(l.mode))
        .show_ui(ui, |ui| {
            for mode in LightingMode::ALL {
                ui.selectable_value(&mut l.mode, mode, mode_label(mode));
            }
        });
    ui.add_space(8.0);

    egui::Grid::new("light")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| match l.mode {
            LightingMode::Off => {}
            LightingMode::Glorious => {
                speed(ui, &mut l.glorious.speed);
                ui.label("Direction");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut l.glorious.direction, Direction::Up, "up");
                    ui.radio_value(&mut l.glorious.direction, Direction::Down, "down");
                });
                ui.end_row();
            }
            LightingMode::Single => {
                brightness(ui, &mut l.single.brightness);
                colors(ui, "Colour", std::slice::from_mut(&mut l.single.color));
            }
            LightingMode::Breathing => {
                brightness(ui, &mut l.breathing.brightness);
                speed(ui, &mut l.breathing.speed);
                ui.label("Colours used");
                ui.add(Slider::new(&mut l.breathing.count, 1..=7));
                ui.end_row();
                let n = l.breathing.count.clamp(1, 7) as usize;
                colors(ui, "Colours", &mut l.breathing.colors[..n]);
            }
            LightingMode::Tail => {
                brightness(ui, &mut l.tail.brightness);
                speed(ui, &mut l.tail.speed);
            }
            LightingMode::SeamlessBreathing => speed(ui, &mut l.seamless.speed),
            LightingMode::Gradient => colors(ui, "Gradient", &mut l.gradient.colors),
            LightingMode::Rave => {
                brightness(ui, &mut l.rave.brightness);
                speed(ui, &mut l.rave.speed);
                colors(ui, "Colours", &mut l.rave.colors);
            }
            LightingMode::Glitch => speed(ui, &mut l.glitch.speed),
            LightingMode::Wave => {
                brightness(ui, &mut l.wave.brightness);
                speed(ui, &mut l.wave.speed);
            }
            LightingMode::BreathingSingle => {
                speed(ui, &mut l.breathing_single.speed);
                colors(
                    ui,
                    "Colour",
                    std::slice::from_mut(&mut l.breathing_single.color),
                );
            }
        });
}

fn mode_label(mode: LightingMode) -> &'static str {
    match mode {
        LightingMode::Off => "Off",
        LightingMode::Glorious => "Glorious",
        LightingMode::Single => "Single colour",
        LightingMode::Breathing => "Breathing",
        LightingMode::Tail => "Tail",
        LightingMode::SeamlessBreathing => "Seamless breathing",
        LightingMode::Gradient => "Gradient",
        LightingMode::Rave => "Rave",
        LightingMode::Glitch => "Glitch",
        LightingMode::Wave => "Wave",
        LightingMode::BreathingSingle => "Breathing single colour",
    }
}

fn brightness(ui: &mut Ui, value: &mut u8) {
    ui.label("Brightness");
    ui.horizontal(|ui| {
        for b in 1..=4 {
            ui.radio_value(value, b, format!("{}%", b as u16 * 25));
        }
    });
    ui.end_row();
}

fn speed(ui: &mut Ui, value: &mut u8) {
    ui.label("Speed");
    ui.add(Slider::new(value, 1..=MAX_SPEED));
    ui.end_row();
}

fn colors(ui: &mut Ui, label: &str, colors: &mut [Rgb]) {
    ui.label(label);
    ui.horizontal(|ui| {
        for c in colors {
            color_button(ui, c);
        }
    });
    ui.end_row();
}

fn color_button(ui: &mut Ui, c: &mut Rgb) {
    let mut rgb = [c.r, c.g, c.b];
    if ui.color_edit_button_srgb(&mut rgb).changed() {
        *c = Rgb::new(rgb[0], rgb[1], rgb[2]);
    }
}

fn buttons_tab(ui: &mut Ui, buttons: &mut Buttons) {
    ui.label(
        RichText::new("Macros aren't supported, but macros set up with other software keep working unless you change that button.")
            .weak(),
    );
    ui.add_space(8.0);
    egui::Grid::new("buttons")
        .num_columns(3)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            for (i, name) in PHYSICAL_BUTTONS.iter().enumerate() {
                ui.label(*name);
                let action = &mut buttons.slots[i];
                ComboBox::from_id_salt(("button", i))
                    .selected_text(action_kind(*action))
                    .show_ui(ui, |ui| {
                        for b in MouseButton::ALL {
                            ui.selectable_value(action, ButtonAction::Mouse(b), b.name());
                        }
                        ui.selectable_value(action, ButtonAction::Dpi, "DPI cycle");
                        let is_key = matches!(action, ButtonAction::Key { .. });
                        if ui.selectable_label(is_key, "keyboard key").clicked() && !is_key {
                            *action = ButtonAction::Key {
                                modifiers: 0,
                                key: 0x04,
                            };
                        }
                    });
                if let ButtonAction::Key { key, .. } = action {
                    ComboBox::from_id_salt(("key", i))
                        .selected_text(key_name(*key))
                        .show_ui(ui, |ui| {
                            // Letters, digits, Enter/Esc/Backspace/Tab/Space, F1-F12.
                            for code in (0x04..=0x2c).chain(0x3a..=0x45) {
                                ui.selectable_value(key, code, key_name(code));
                            }
                        });
                } else {
                    ui.label("");
                }
                ui.end_row();
            }
        });
}

fn action_kind(action: ButtonAction) -> String {
    match action {
        ButtonAction::Key { .. } => "keyboard key".into(),
        other => other.describe(),
    }
}
