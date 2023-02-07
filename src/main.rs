#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{
    egui::{
        self,
        plot::{BarChart, Plot, Points},
    },
    epaint::Color32,
};

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1280.0, 720.0)),
        ..Default::default()
    };
    eframe::run_native("FOGT", options, Box::new(|cc| Box::new(MyEguiApp::new(cc))));
}

struct MyEguiApp {
    particles_pos: Vec<[f64; 2]>,
    particles_previous_pos: Vec<[f64; 2]>,
    particles_speed: Vec<f64>,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn simulation(&mut self, time: f64) {
        self.particles_pos[0][0] = time.sin();
    }
}

impl Default for MyEguiApp {
    fn default() -> Self {
        Self {
            particles_pos: vec![[0.5, 0.2]],
            particles_previous_pos: Vec::new(),
            particles_speed: Vec::new(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Widok cząsteczek
                let markers_plot = Plot::new("markers_demo")
                    .view_aspect(1.0)
                    .width(700.0)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .allow_zoom(false)
                    .allow_boxed_zoom(false);
                markers_plot.show(ui, |plot_ui| {
                    plot_ui.points(Points::new(self.particles_pos.clone()).radius(5.0));
                });

                ui.vertical(|ui| {
                    // Histogram prędkości
                    ui.heading("Rozkład prędkości");

                    let chart = BarChart::new(Vec::new()).color(Color32::LIGHT_BLUE);
                    Plot::new("predkosc")
                        .view_aspect(1.0)
                        .width(325.0)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .allow_zoom(false)
                        .allow_boxed_zoom(false)
                        .show(ui, |plot_ui| plot_ui.bar_chart(chart));

                    // Histogram energii
                    ui.heading("Rozkład energii");

                    let chart = BarChart::new(Vec::new()).color(Color32::LIGHT_GREEN);
                    Plot::new("energia")
                        .view_aspect(1.0)
                        .width(325.0)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .allow_zoom(false)
                        .allow_boxed_zoom(false)
                        .show(ui, |plot_ui| plot_ui.bar_chart(chart));
                });

                // Opcje
                ui.vertical(|ui| {
                    ui.heading("Opcje");
                });
            });

            let time = ui.input().time;
            self.simulation(time);
            ui.ctx().request_repaint()
        });
    }
}
