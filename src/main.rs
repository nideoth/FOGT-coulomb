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
    time_multiplier: f32,
    velocity_precision: f32,
    energy_precision: f32,
    rng: ThreadRng,
    /* Parametry wstawiania nowych cząsteczek myszką. */
    user_particle_input_state: UserParticleInputState,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        return Self {
            /*
            particles: {
                let mut rng = rand::thread_rng();
                std::iter::from_fn(move || {
                    Some(Particle::new(
                        rng.sample(Uniform::new(0.0, 1.0)),
                        rng.sample(Uniform::new(0.0, 1.0)),
                        rng.sample(Uniform::new(-1.0, 1.0)),
                        rng.sample(Uniform::new(0.0, 1.0)),
                    ))
                })
            }
            .take(4)
            .collect(),
            */
            /*
            particles: vec![
                Particle::new(0.5, 0.5, 0.9, 0.5),
                Particle::new(0.6, 0.5, -0.9, 0.5),
            ],
            */
            particles: vec![],
            time_multiplier: 1.0,
            velocity_precision: 0.2,
            energy_precision: 1.0,
            rng: rand::thread_rng(),
            user_particle_input_state: UserParticleInputState{
                count: 1,
                charge: 0.5,
                mass: 0.5,
            },
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

    /* Dodawanie cząsteczek przez kliknięcie myszką. */
    fn add_user_particles(&mut self, x: f32, y: f32, input_state: UserParticleInputState) {
        for _ in 0..input_state.count {
            /* Jeśli jedna cząsteczka, wstawiamy ją dokładnie tam, gdzie jest kursor. 
             * Jeśli więcej, to dodajemy pewien rozrzut, bo inaczej wszystkie by się pokryły. */
            let radius = self.rng.sample(Uniform::new(0.0, 1.0)) * u32::min(input_state.count - 1, 1) as f32 * 0.1;
            let angle = self.rng.sample(Uniform::new(0.0, std::f32::consts::PI * 2.0));
            let dx = radius * f32::cos(angle);
            let dy = radius * f32::sin(angle);

            if Particle::valid(x + dx, y + dy, input_state.charge, input_state.mass) {
                self.particles.push(Particle::new(x + dx, y + dy, input_state.charge, input_state.mass));
            }
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /* Potrzebne do obliczania współrzędnych przy dodawaniu cząsteczek myszką:
         * wartości współrzędnych kursora w układzie współrzędnych wykresu.*/
        let mut particle_plot_pointer_coordinates = None;
    
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Widok cząsteczek
                let markers_plot = Plot::new("markers_demo")
                    .view_aspect(1.0)
                    .width(700.0)
                    /* TODO: to powinien być kwadrat, ale wtedy nie mieści mi się na ekranie... */
                    //.height(700.0)
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
                    for p in &self.particles {
                        /* Dozwolone przedziały masy i ładunku są opisane w Particle::new(). */

                        /* Moim zdaniem lepiej, żeby kolor był bezwzględny, a nie skalowany do
                         * maksymalnego ładunku wśród wszystkich cząsteczek, bo jak byśmy chcieli
                         * dodać nowe cząsteczki w trakcie symulacji, to trzeba by było wszystko od
                         * nowa przeliczać jeśli zmieni się maksimum. */
                        let color_value = if p.charge >= 0.0 {
                            /* "Casting from a float to an integer will round the float towards zero". */
                            Color32::from_rgb((255.0 * p.charge) as u8, 0, 0)
                        } else {
                            Color32::from_rgb(0, 0, (-255.0 * p.charge) as u8)
                        };

                        /* Analogiczny komentarz jak dla ładunku. */
                        let radius = 7.0 * p.mass + 3.0;
                        plot_ui.points(
                            Points::new([p.position.x as f64, p.position.y as f64])
                                .radius(radius)
                                .color(color_value),
                        );
                    }

                    particle_plot_pointer_coordinates = plot_ui.pointer_coordinate();
                });

                ui.vertical(|ui| {
                    // Histogram prędkości
                    ui.heading("Rozkład prędkości");

                    /* Przykład tego co miałem na myśli gdy mówiłem o tym, żeby ten histogram był
                     * posortowany. */

                    let mut velocities: Vec<_> = self.particles
                        .iter()
                        .map(|p| if p.velocity.magnitude().is_finite() { p.velocity.magnitude() } else { 0.0 } )
                        .collect();

                    /* Floaty nie implementują `Ord` bo NaN != NaN. */
                    velocities.sort_by(|a, b| a.total_cmp(b));

                    let bars: Vec<_> = velocities
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| Bar::new((i + 1) as f64, v as f64))
                        .collect();

                    let chart_size = 325.0; /* Szerokość i wysokość. */
                    let y_range = 6.0;

                    let chart = BarChart::new(bars)
                        .width(1.0)
                        .color(Color32::LIGHT_BLUE);

                    Plot::new("predkosc")
                        .width(chart_size)
                        .height(chart_size)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .allow_zoom(false)
                        .allow_boxed_zoom(false)
                        .include_y(0)
                        .include_y(y_range)
                        .show(ui, |plot_ui| plot_ui.bar_chart(chart));


                    /*
                    let mut bars: Vec<Bar> = Vec::new();
                    let values: Vec<f32> = self
                        .particles
                        .iter()
                        .map(|p| (p.velocity[0].powi(2) + p.velocity[1].powi(2)).sqrt())
                        .map(|v| (v / self.velocity_precision).floor() * self.velocity_precision)
                        .collect();
                    for v in values {
                        let bar = Bar::new((v + self.velocity_precision / 2.0) as f64, 1.0);
                        let index = bars.iter().position(|b| b.argument == bar.argument);
                        match index {
                            None => bars.push(bar),
                            Some(i) => bars.get_mut(i).unwrap().value += 1.0,
                        }
                    }
                    */

                    /*
                    let chart = BarChart::new(bars)
                        .width(self.velocity_precision as f64)
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
                        /* Tych dwóch właśnie chyba powinno nie być, skoro ustawiamy na sztywno zakres:
                        .auto_bounds_x()
                        .auto_bounds_y()
                        */
                        .show(ui, |plot_ui| plot_ui.bar_chart(chart));
                        */

                    // Histogram energii
                    ui.heading("Rozkład energii");

                    let mut bars: Vec<Bar> = Vec::new();
                    let values: Vec<f32> = self
                        .particles
                        .iter()
                        .map(|p| {
                            let velocity = (p.velocity[0].powi(2) + p.velocity[1].powi(2)).sqrt();
                            let mass = p.mass;
                            mass * velocity.powi(2) * 0.5
                        })
                        .map(|v| (v / self.energy_precision).floor() * self.energy_precision)
                        .collect();
                    for v in values {
                        let bar = Bar::new((v + self.energy_precision / 2.0) as f64, 1.0);
                        let index = bars.iter().position(|b| b.argument == bar.argument);
                        match index {
                            None => bars.push(bar),
                            Some(i) => bars.get_mut(i).unwrap().value += 1.0,
                        }
                    }

                    let chart = BarChart::new(bars)
                        .width(self.energy_precision as f64)
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

                    ui.add_space(16.0);
                    ui.label("Mnożnik czasu");
                    ui.add(egui::Slider::new(&mut self.time_multiplier, 0.0..=1.0));

                    ui.add_space(16.0);
                    ui.label("Precyzja histogramu prędkości");
                    ui.add(egui::Slider::new(&mut self.velocity_precision, 0.2..=2.0));

                    ui.add_space(16.0);
                    ui.label("Precyzja histogramu energii");
                    ui.add(egui::Slider::new(&mut self.energy_precision, 0.4..=4.0));

                    /* Opcje wstawiania cząsteczek myszką. */
                    ui.heading("Wstawianie cząsteczek");

                    ui.add_space(16.0);
                    ui.add(egui::Slider::new(&mut self.user_particle_input_state.count, 1..=10).text("Ilość").clamp_to_range(false));

                    ui.add_space(16.0);
                    ui.add(egui::Slider::from_get_set(
                        std::ops::RangeInclusive::new(-1.0, 1.0),
                        |x| {if let Some(x) = x { self.user_particle_input_state.charge = x as f32; } self.user_particle_input_state.charge as f64 }
                    ).text("Ładunek").clamp_to_range(true));

                    ui.add_space(16.0);
                    ui.add(egui::Slider::from_get_set(
                        std::ops::RangeInclusive::new(0.1, 1.0),
                        |x| {if let Some(x) = x { self.user_particle_input_state.mass = x as f32; } self.user_particle_input_state.mass as f64 }
                    ).text("Masa").clamp_to_range(true));



                    ui.add_space(16.0);
                    if ui.button("Przywróć domyślne").clicked() {
                        self.time_multiplier = 1.0;
                        self.velocity_precision = 0.2;
                        self.energy_precision = 1.0;
                    }

                    ui.add_space(16.0);
                    ui.label("Motyw");
                    egui::widgets::global_dark_light_mode_buttons(ui);
                });
            });

            /* Dodawanie cząsteczek przez kliknięcie. */
            if ui.input().pointer.primary_clicked()  {
                if let Some(egui::widgets::plot::PlotPoint{x, y}) = particle_plot_pointer_coordinates {
                    self.add_user_particles(x as f32, y as f32, self.user_particle_input_state);
                }
            }


            let d_time = ui.input().stable_dt * self.time_multiplier;
            self.simulation(d_time);
            ui.ctx().request_repaint()
        });
    }
}

/* Parametry wstawiania nowych cząsteczek myszką. */
#[derive(Copy, Clone)]
struct UserParticleInputState {
    count: u32,
    charge: f32,
    mass: f32,
}
