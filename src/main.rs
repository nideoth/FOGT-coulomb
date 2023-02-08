#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::needless_return)]

mod particle;

use particle::Particle;
use rand::distributions::Uniform;
use rand::prelude::*;

use eframe::{
    egui::{
        self,
        plot::{Bar, BarChart, Plot, Points},
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
    particles: Vec<Particle>,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        return Self {
            particles: {
                let mut rng = rand::thread_rng();
                std::iter::from_fn(move || {
                    Some(Particle::new(
                        rng.sample(Uniform::new(0.0, 1.0)),
                        rng.sample(Uniform::new(0.0, 1.0)),
                        rng.sample(Uniform::new(-5.0, 5.0)),
                        rng.sample(Uniform::new(0.0, 10.0)),
                    ))
                })
            }
            .take(10)
            .collect(),
        };
    }

    fn simulation(&mut self, d_time: f32) {
        /* Wypadkowe siły dla każdej cząsteczki w wektorze `particles`.
         *
         * Przy przekazywaniu cząsteczek do `net_electrostatic_force` musimy wyrzucić tą, dla
         * której liczymy siłę, żeby nie liczyć oddziaływania elektrostatycznego niej samej ze sobą. */
        let forces = self
            .particles
            .iter()
            .enumerate()
            .map(|(i, p)| {
                p.net_electrostatic_force(
                    self.particles
                        .iter()
                        .enumerate()
                        .filter(|&(ii, _)| ii != i)
                        .map(|(_, p)| p),
                ) + p.gravitational_force()
            })
            .collect::<Vec<_>>();

        self.particles
            .iter_mut()
            .zip(forces.iter())
            .for_each(|(p, f)| p.apply_force(*f, d_time));
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
                    .height(600.0)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .allow_zoom(false)
                    .allow_boxed_zoom(false)
                    /* To jest potrzebne, żeby skala wykresu nie próbowała się ciągle dopasowywać
                     * do rozmieszczenia cząsteczek. */
                    .include_x(0.0)
                    .include_x(1.0)
                    .include_y(0.0)
                    .include_y(1.0);
                markers_plot.show(ui, |plot_ui| {
                    /* TODO: zróbmy żeby ładunki dodatnie i ujemne były w różnych kolorach, i może
                     * wielkość punktów zależną od ich masy. */
                    plot_ui.points(
                        Points::new(
                            self.particles
                                .iter()
                                .map(|p| [p.position.x as f64, p.position.y as f64])
                                .collect::<Vec<_>>(),
                        )
                        .radius(5.0),
                    );
                });

                ui.vertical(|ui| {
                    // Histogram prędkości
                    ui.heading("Rozkład prędkości");

                    let precision = 0.2;

                    let mut bars: Vec<Bar> = Vec::new();
                    let values: Vec<f32> = self
                        .particles
                        .iter()
                        .map(|p| (p.velocity[0].powi(2) + p.velocity[1].powi(2)).sqrt())
                        .map(|v| (v / precision).floor() * precision)
                        .collect();
                    for v in values {
                        let bar = Bar::new((v + precision / 2.0) as f64, 1.0);
                        let index = bars.iter().position(|b| b.argument == bar.argument);
                        match index {
                            None => bars.push(bar),
                            Some(i) => bars.get_mut(i).unwrap().value += 1.0,
                        }
                    }

                    let chart = BarChart::new(bars)
                        .width(precision as f64)
                        .color(Color32::LIGHT_BLUE);
                    Plot::new("predkosc")
                        .width(325.0)
                        .height(325.0)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .allow_zoom(false)
                        .allow_boxed_zoom(false)
                        .include_x(0)
                        .include_x(8.0)
                        .include_y(0)
                        .include_y(4.0)
                        .auto_bounds_x()
                        .auto_bounds_y()
                        .show(ui, |plot_ui| plot_ui.bar_chart(chart));

                    // Histogram energii
                    ui.heading("Rozkład energii");

                    let precision = 1.0;

                    let mut bars: Vec<Bar> = Vec::new();
                    let values: Vec<f32> = self
                        .particles
                        .iter()
                        .map(|p| {
                            let velocity = (p.velocity[0].powi(2) + p.velocity[1].powi(2)).sqrt();
                            let mass = p.mass;
                            mass * velocity.powi(2) * 0.5
                        })
                        .map(|v| (v / precision).floor() * precision)
                        .collect();
                    for v in values {
                        let bar = Bar::new((v + precision / 2.0) as f64, 1.0);
                        let index = bars.iter().position(|b| b.argument == bar.argument);
                        match index {
                            None => bars.push(bar),
                            Some(i) => bars.get_mut(i).unwrap().value += 1.0,
                        }
                    }

                    let chart = BarChart::new(bars)
                        .width(precision as f64)
                        .color(Color32::LIGHT_GREEN);
                    Plot::new("energia")
                        .width(325.0)
                        .height(325.0)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .allow_zoom(false)
                        .allow_boxed_zoom(false)
                        .include_x(0)
                        .include_x(32.0)
                        .include_y(0)
                        .include_y(4.0)
                        .auto_bounds_x()
                        .auto_bounds_y()
                        .show(ui, |plot_ui| plot_ui.bar_chart(chart));
                });

                // Opcje
                ui.vertical(|ui| {
                    ui.heading("Opcje");
                });
            });

            let d_time = ui.input().stable_dt;
            self.simulation(d_time);
            ui.ctx().request_repaint()
        });
    }
}
