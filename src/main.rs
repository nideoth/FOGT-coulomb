#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::needless_return)]

mod particle;

use particle::{Particle, Vect};
use rand::distributions::Uniform;
use rand::prelude::*;
extern crate nalgebra as na;

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

/* TODO: dodać przycisk do restartu symulacji bez wyłączania programu. */
/* TODO: trzeba wymyślić lepszy schemat kolorów dla cząsteczek, bo im mniejszy ładunek tym bardziej
 * czarne one są, i nie widać ich na tle wykresu. */
/* TODO: poprawić layout UI, żeby wszystkie wykresy się mieściły (zakładki?) */



/* Co ma się dziać przy kliknięciu: dodawanie cząsteczek lub śledzenie zaznaczonej cząsteczki. */
enum ClickAction { Add, Track }

struct MyEguiApp {
    particles: Vec<Particle>,
    time_multiplier: f32,
    velocity_precision: f32,
    energy_precision: f32,
    rng: ThreadRng,
    /* Parametry wstawiania nowych cząsteczek myszką. */
    user_particle_input_state: UserParticleInputState,
    /* ID następnej stworzonej cząsteczki. */
    next_particle_id: u32,
    /* Aktualnie śledzona cząsteczka. */
    tracked_particle: Option<TrackedParticle>,
    click_action: ClickAction,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        return Self {
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
            next_particle_id: 0,
            tracked_particle: None,
            click_action: ClickAction::Add,
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
            .map(|p| {
                p.net_electrostatic_force(
                    self.particles
                        .iter()
                        .filter(|p2| p.id != p2.id)
                ) + p.gravitational_force()
            })
            .collect::<Vec<_>>();

        self.particles
            .iter_mut()
            .zip(forces.iter())
            .for_each(|(p, f)| p.apply_force(*f, d_time));

        /* Zapisujemy dane śledzonej cząsteczki z tej instancji symulacji do narysowania wykresów. */
        if let Some(ref mut tracked_particle) = self.tracked_particle {
            if let Some(particle) = self.particles.iter().find(|p| p.id == tracked_particle.id) {
                if tracked_particle.path.len() == TrackedParticle::DATA_POINT_COUNT_PATH {
                    tracked_particle.path.pop_front();
                }

                if tracked_particle.velocity.len() == TrackedParticle::DATA_POINT_COUNT_VELOCITY {
                    tracked_particle.velocity.pop_front();
                }

                if tracked_particle.acceleration.len() == TrackedParticle::DATA_POINT_COUNT_ACCELERATION {
                    tracked_particle.acceleration.pop_front();
                }

                tracked_particle.path.push_back(particle.position);
                tracked_particle.velocity.push_back(particle.velocity);
                tracked_particle.acceleration.push_back(particle.acceleration);
            }
        }
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
                self.particles.push(Particle::new(self.next_particle_id, x + dx, y + dy, input_state.charge, input_state.mass));
                self.next_particle_id += 1;
            }
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /* Potrzebne do obliczania współrzędnych przy dodawaniu cząsteczek myszką:
         * wartości współrzędnych kursora w układzie współrzędnych wykresu.*/
        let mut particle_plot_pointer_coordinates = None;

        /* Id cząsteczki aktualnie pod kursorem (może być inna niż aktualnie śledzona). */
        let mut selected_particle_id: Option<u32> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            /* Wielkość okienka z symulacją. */
            let simulation_plot_size = 500.0;
            /* Wielkość pozostałych wykresów. */
            let plot_size = 200.0;

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    {
                        /* Cząsteczki. */
                        let markers_plot = Plot::new("markers_demo")
                            .view_aspect(1.0)
                            .width(simulation_plot_size)
                            .height(simulation_plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .include_x(0.0)
                            .include_x(1.0)
                            .include_y(0.0)
                            .include_y(1.0);

                        ui.heading("Symulacja");

                        markers_plot.show(ui, |plot_ui| {
                            particle_plot_pointer_coordinates = plot_ui.pointer_coordinate();

                            /* Szukamy indeksu cząsteczki pod kursorem. */
                            if let Some(particle_plot_pointer_coordinates) = particle_plot_pointer_coordinates {
                                let particle_plot_pointer_coordinates = na::Vector2::<f64>::new(particle_plot_pointer_coordinates.x, particle_plot_pointer_coordinates.y);

                                /* Jeśli kursor znajduje się w co najwyżej takiej odległości od środka
                                 * cząsteczki, to uznajemy, że jest na cząsteczce. */
                                let selection_radius = 0.0235;

                                /* Szukamy cząsteczki najbliżej kursora i patrzymy, czy jest w promieniu. */
                                selected_particle_id = self.particles.iter()
                                    .map(|p| (p.id, (p.position.cast::<f64>() - particle_plot_pointer_coordinates).magnitude()))
                                    .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
                                    .filter(|(_, d)| d <= &selection_radius)
                                    .map(|(id, _)| id);

                            }


                            for p in &self.particles {
                                /* Dozwolone przedziały masy i ładunku są opisane w Particle::new(). */

                                /* Kolor jest skalowany do dozwolonego przedziału ładunku, a nie do
                                 * maksymalnego ładunku wśród wszystkich cząsteczek, bo jak dodajemy
                                 * nowe cząsteczki w trakcie symulacji, to trzeba by było wszystko od
                                 * nowa przeliczać jeśli zmieni się maksimum. */
                                let color_value = 
                                    /* Śledzona cząsteczka ma się wyróżniać. */
                                    if self.tracked_particle.is_some() && self.tracked_particle.as_ref().unwrap().id == p.id {
                                        Color32::from_rgb(0, 255, 255)
                                    /* Zaznaczona też. */
                                    } else if selected_particle_id.is_some() && selected_particle_id.unwrap() == p.id {
                                        Color32::from_rgb(255, 255, 0)
                                    } else if p.charge >= 0.0 {
                                        /* "Casting from a float to an integer will round the float towards zero". */
                                        Color32::from_rgb(255, 255 - (255.0 * p.charge) as u8, 255 - (255.0 * p.charge) as u8)
                                    } else {
                                        Color32::from_rgb(255 - (-255.0 * p.charge) as u8, 255 - (-255.0 * p.charge) as u8, 255)
                                    };

                                /* Analogiczny komentarz jak dla ładunku. */
                                let radius = 7.0 * p.mass + 3.0;
                                plot_ui.points(
                                    Points::new([p.position.x as f64, p.position.y as f64])
                                        .radius(radius)
                                        .color(color_value),
                                );
                            }

                        });
                    }

                    /* Opcje */
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading("Symulacja");

                            ui.label("Szykość symulacji");
                            /* Fixed decimals, bo inaczej całe UI się przesuwa gdy zmienia się
                             * liczba cyfr po przecinku. */
                            ui.add(egui::Slider::new(&mut self.time_multiplier, 0.0..=1.0).fixed_decimals(2));

                            if ui.button("Nowa symulacja").clicked() {
                                /* TODO */
                            }
                        });

                        ui.vertical(|ui| {
                            ui.heading("Interakcje");

                            ui.horizontal(|ui| {
                                if ui.button("Wstawianie").clicked() { self.click_action = ClickAction::Add; }
                                if ui.button("Śledzenie").clicked() { self.click_action = ClickAction::Track; }
                            });

                            ui.add(egui::Slider::new(&mut self.user_particle_input_state.count, 1..=10).text("Ilość").clamp_to_range(false));

                            ui.add(egui::Slider::from_get_set(
                                std::ops::RangeInclusive::new(-1.0, 1.0),
                                |x| { 
                                    if let Some(x) = x { 
                                        self.user_particle_input_state.charge = x as f32; 
                                    } 
                                    self.user_particle_input_state.charge as f64 
                                }
                            ).text("Ładunek").clamp_to_range(true)
                                .custom_formatter(|value, _| format!("{:+.2}", value))
                            );

                            ui.add(egui::Slider::from_get_set(
                                std::ops::RangeInclusive::new(0.01, 1.0),
                                |x| { 
                                    if let Some(x) = x { 
                                        self.user_particle_input_state.mass = x as f32; 
                                    } 
                                    self.user_particle_input_state.mass as f64 
                                }
                            ).text("Masa").clamp_to_range(true).fixed_decimals(2));

                        });

                        ui.vertical(|ui| {
                            ui.heading("Wyświetlanie");


                            ui.label("Motyw");
                            egui::widgets::global_dark_light_mode_buttons(ui);

                        });
                    });
                });

                ui.vertical(|ui| {
                    {
                        /* Prawdziwy histogram prędkości. */
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
                            

                        ui.heading("Histogram prędkości");

                        let chart = BarChart::new(bars)
                            .width(self.velocity_precision as f64)
                            .color(Color32::LIGHT_BLUE);
                        Plot::new("predkosc")
                            .width(plot_size)
                            .height(plot_size)
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

                        ui.label("Precyzja histogramu prędkości");
                        ui.add(egui::Slider::new(&mut self.velocity_precision, 0.2..=2.0));
                    }

                    ui.add_space(16.0);

                    {
                        /* Histogram energii. */
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

                        ui.heading("Histogram energii");

                        let chart = BarChart::new(bars)
                            .width(self.energy_precision as f64)
                            .color(Color32::LIGHT_GREEN);
                        Plot::new("energia")
                            .width(plot_size)
                            .height(plot_size)
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

                        ui.label("Precyzja histogramu energii");
                        ui.add(egui::Slider::new(&mut self.energy_precision, 0.4..=4.0));
                    }

                });

                ui.vertical(|ui| {
                    {
                        /* Pole wektorowe siły elektrostatycznej. */

                        let vector_field = Plot::new("vector_field")
                            .view_aspect(1.0)
                            .width(plot_size)
                            .height(plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .include_x(0.0)
                            .include_x(1.0)
                            .include_y(0.0)
                            .include_y(1.0)
                            .show_axes([false, false]);

                        ui.heading("Pole elektryczne");

                        /* Ile wektorów chcemy mieć w każdym wymiarze. */
                        let resolution = 8;
                        let arrow_length = 0.1;

                        let mut arrow_origins = Vec::with_capacity(resolution);
                        let mut arrow_tips = Vec::with_capacity(resolution);

                        for x in 0..resolution {
                            for y in 0..resolution {
                                let [x, y] = [(x as f64 + 0.5)/resolution as f64, (y as f64 + 0.5)/resolution as f64];
                                let mut force = Particle::new(u32::MAX, x as f32, y as f32, 1.0, 0.5)
                                    .net_electrostatic_force(self.particles.iter());

                                if force.magnitude() != 0.0 {
                                    force = force.normalize() * arrow_length;
                                }

                                arrow_origins.push([x, y]);
                                arrow_tips.push([x + force.x as f64, y + force.y as f64]);
                            }
                        }


                        vector_field.show(ui, |plot_ui| {
                            plot_ui.arrows(
                                egui::widgets::plot::Arrows::new(egui::widgets::plot::PlotPoints::from(arrow_origins), egui::widgets::plot::PlotPoints::from(arrow_tips))
                                .color(Color32::from_rgb(255, 255, 255))
                            )
                        });

                    } 

                    {
                        /* "Histogram" prędkości. */ 
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

                        let chart = BarChart::new(bars)
                            .width(1.0)
                            .color(Color32::LIGHT_BLUE);

                        ui.heading("Rozkład prędkości");

                        Plot::new("predkosc2")
                            .width(plot_size)
                            .height(plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .show_axes([false, false])
                            .show(ui, |plot_ui| plot_ui.bar_chart(chart));
                    }

                    {
                        /* Wykres środka ciężkości. */

                        /* Jak się okazuje, nie ma chyba analogicznej wielkości dla ładunku, którą
                         * można policzyć bez ogromnego wysiłku. */
                        let center_of_mass_plot = Plot::new("center_of_mass")
                            .view_aspect(1.0)
                            .width(plot_size)
                            .height(plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .include_x(0.0)
                            .include_x(1.0)
                            .include_y(0.0)
                            .include_y(1.0)
                            .show_axes([false, false]);

                        ui.heading("Środek masy");

                        center_of_mass_plot.show(ui, |plot_ui| {
                            if let Some(center_of_mass) = Particle::center_of_mass(self.particles.iter()) {
                                plot_ui.points(
                                    Points::new([center_of_mass.x as f64, center_of_mass.y as f64])
                                        .radius(4.0)
                                        .color(Color32::from_rgb(255, 255, 255))
                                );
                            }

                            plot_ui.text(
                                egui::widgets::plot::Text::new(
                                    egui::widgets::plot::PlotPoint{x: 0.0, y: 1.0},
                                    format!("Całkowita masa: {}", self.particles.iter().map(|p| p.mass).sum::<f32>())
                                )
                                .anchor(egui::Align2::LEFT_TOP)
                                .color(Color32::from_rgb(255, 255, 255))
                            );
                        });

                    }

                    


                });

                ui.vertical(|ui| {
                    /* Wykresy dla śledzonej cząsteczki. */
                    if let Some(ref tracked_particle) = self.tracked_particle {
                        /* Ścieżka ruchu. */

                        ui.heading("Tor ruchu");

                        let path_plot = Plot::new("tracked_path")
                            .view_aspect(1.0)
                            .width(plot_size)
                            .height(plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .include_x(0.0)
                            .include_x(1.0)
                            .include_y(0.0)
                            .include_y(1.0)
                            .show_axes([false, false]);

                        path_plot.show(ui, |plot_ui| {
                            plot_ui.line(
                                egui::widgets::plot::Line::new(egui::widgets::plot::PlotPoints::from_iter(
                                    tracked_particle.path
                                    .iter()
                                    .map(|a| [a.x as f64, a.y as f64])
                                )).color(Color32::from_rgb(255, 255, 255))
                            )
                        });

                        /* Prędkość. */

                        ui.heading("Wartość prędkości");

                        let velocity_plot = Plot::new("tracked_velocity")
                            .view_aspect(1.0)
                            .width(plot_size)
                            .height(plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .include_x(0.0)
                            .include_x(1.0)
                            .show_axes([false, true]);

                        velocity_plot.show(ui, |plot_ui| {
                            plot_ui.line(
                                egui::widgets::plot::Line::new(egui::widgets::plot::PlotPoints::from_iter(
                                    tracked_particle.velocity
                                    .iter()
                                    .zip(0..TrackedParticle::DATA_POINT_COUNT_VELOCITY)
                                    .map(|(y, x)| [x as f64 / TrackedParticle::DATA_POINT_COUNT_VELOCITY as f64, y.magnitude() as f64])
                                )).color(Color32::from_rgb(255, 255, 255))
                            )
                        });

                        /* Przyspieszenie. */

                        ui.heading("Wartość przyspieszenia");

                        let acceleration_plot = Plot::new("tracked_acceleration")
                            .view_aspect(1.0)
                            .width(plot_size)
                            .height(plot_size)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .include_x(0.0)
                            .include_x(1.0)
                            .show_axes([false, true]);

                        acceleration_plot.show(ui, |plot_ui| {
                            plot_ui.line(
                                egui::widgets::plot::Line::new(egui::widgets::plot::PlotPoints::from_iter(
                                    tracked_particle.acceleration
                                    .iter()
                                    .zip(0..TrackedParticle::DATA_POINT_COUNT_ACCELERATION)
                                    .map(|(y, x)| [x as f64 / TrackedParticle::DATA_POINT_COUNT_ACCELERATION as f64, y.magnitude() as f64])
                                )).color(Color32::from_rgb(255, 255, 255))
                            )
                        });
                    }
                });
            });

            /* Dodawanie cząsteczek przez kliknięcie lub śledzenie cząsteczki. */
            if ui.input().pointer.primary_clicked() {
                match self.click_action {
                    ClickAction::Add => {
                        if let Some(egui::widgets::plot::PlotPoint{x, y}) = particle_plot_pointer_coordinates {
                            self.add_user_particles(x as f32, y as f32, self.user_particle_input_state);
                        }
                    },
                    ClickAction::Track => {
                        if let Some(id) = selected_particle_id {
                            self.tracked_particle = Some(TrackedParticle::new(id));
                        }
                    }
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

/* Wszystkie punkty danych potrzebne do zrobienia wykresów dla śledzonej cząsteczki. */
#[derive(Debug)]
struct TrackedParticle {
    id: u32,
    path: std::collections::VecDeque<Vect>,
    velocity: std::collections::VecDeque<Vect>,
    acceleration: std::collections::VecDeque<Vect>,
}

impl TrackedParticle {
    /* Dla ilu chwil czasu chcemy trzymać wartości. */
    const DATA_POINT_COUNT_PATH: usize = 1024;
    const DATA_POINT_COUNT_VELOCITY: usize = 256;
    const DATA_POINT_COUNT_ACCELERATION: usize = 256;

    fn new(id: u32) -> Self {
        return Self{
            id,
            path: std::collections::VecDeque::with_capacity(Self::DATA_POINT_COUNT_PATH),
            velocity: std::collections::VecDeque::with_capacity(Self::DATA_POINT_COUNT_VELOCITY),
            acceleration: std::collections::VecDeque::with_capacity(Self::DATA_POINT_COUNT_ACCELERATION),
        };
    }
}
