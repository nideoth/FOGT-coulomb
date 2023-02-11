extern crate nalgebra as na;
use core::ops::Deref;

/* Zwykły wektor 2D, żeby nie pisać tego tyle razy. */
type Vect = na::Vector2<f32>;

pub struct Particle {
    pub position: Vect,
    pub velocity: Vect,
    /* Cząsteczki mają oddziaływać elektrostatycznie i grawitacyjnie. */
    pub charge: f32,
    pub mass: f32,
}

impl Particle {
    pub fn new(pos_x: f32, pos_y: f32, charge: f32, mass: f32) -> Self {
    /* Aby ustalić skalę wszystkich wielkości w symulacji i dobrze ustawić stałe,
     * wszystkie te wartości muszą być z konkretnych przedziałów. */
        assert!(Self::valid(pos_x, pos_y, charge, mass));

        return Self {
            position: [pos_x, pos_y].into(),
            velocity: [0.0, 0.0].into(),
            charge,
            mass,
        };
    }

    pub fn valid(pos_x: f32, pos_y: f32, charge: f32, mass: f32) -> bool {
        return
            0.0 <= pos_x && pos_x <= 1.0 &&
            0.0 <= pos_y && pos_y <= 1.0 &&
            -1.0 <= charge && charge <= 1.0 &&
            0.0 < mass && mass <= 1.0;
    }

    /* Wektor siły oddziaływania elektrostatycznego z cząsteczką `other`. */
    pub fn electrostatic_force(&self, other: &Particle) -> Vect {
        /* Wartości stałych możemy raczej dobrać na wyczucie,
         * bo wszystkie wielkości fizyczne w tej symulacji są
         * bez jednostek. */
        const ELECTRO_K: f32 = 1.5;

        let r = self.position - other.position;
        let r_len_sq = r.magnitude_squared();

        /* `r_len_sq` może być zero, gdy dwie cząsteczki się na siebie nałożą.
         * Dla bardzo małych `r_len_sq` spada numeryczna precyzja operacji
         * na floatach, dlatego dostatecznie małe wartości powinniśmy traktować jak 0
         * (bez tego cząsteczki odlatują na koniec świata w niektórych symulacjach). */
        const EPS: f32 = 0.0001;

        /* Wszystkie wartości `r_len_sq`, które nie są skończone (czyli NaN albo nieskończoność)
         * musimy zignorować. */
        if !r_len_sq.is_finite() || r_len_sq < EPS {
            return Vect::zeros();
        } else {
            return ELECTRO_K * self.charge * other.charge * r / r_len_sq;
        }
    }

    /* TODO: może współczynniki siły elektrostatycznej i grawitacyjnej mogłyby być zmieniane przez
     * użytkownika w trakcie symulacji? */

    /* Elektrostatyczna siła wypadkowa działająca na `self`, czyli, suma sił
     * oddziaływań elektrostatycznych z każdą cząsteczką z `particles`. */
    pub fn net_electrostatic_force(
        &self,
        particles: impl Iterator<Item = impl Deref<Target = Particle>>,
    ) -> Vect {
        return particles.fold(Vect::zeros(), |acc, p| acc + self.electrostatic_force(&*p));
    }

    /* Wektor siły grawitacyjnej. */
    pub fn gravitational_force(&self) -> Vect {
        /* Tak jak wcześniej, wszystkie stałe można zastąpić jedną, więc
         * grawitacja będzie po prostu proporcjonalna do masy */
        const GRAVITY_K: f32 = 8.0;

        return Vect::from([0.0, -GRAVITY_K * self.mass]);
    }

    /* Uaktualnia prędkość i pozycję `self` pod wpływem działania siły `force`
     * przez czas `d_time` (to `d_time` to jest taka jakby różniczka czasu). */
    pub fn apply_force(&mut self, force: Vect, d_time: f32) {
        /* Współczynnik sił oporu ruchu. */
        const DRAG_K: f32 = 0.1;

        /* Dla dostatecznie małych prędkości, opór przestaje działać i cząsteczka dalej już
         * nie spowalnia, tylko utrzymuje stałą szybkość. Nie wiem czemu tak jest, pewnie błędy
         * precyzji jak zawsze; jak starczy czasu to coś się z tym zrobi. */
        let drag_force = if self.velocity == Vect::zeros() {
            Vect::zeros()
        } else {
            /* Opór powietrza jest proporcjonalny do v^2, o przeciwnym zwrocie. */
            if !self.velocity.magnitude_squared().is_finite() {
                Vect::zeros()
            } else {
                DRAG_K * self.velocity.magnitude_squared() * (-self.velocity.normalize())
            }
        };

        /* Zakładamy, że przyspieszenie jest stałe w przedziale czasu `d_time`. */
        let acceleration = (force + drag_force) / self.mass;

        /* Zmiana prędkości to pole pod wykresem a(t). */
        let d_velocity = acceleration * d_time;

        /* Zmiana położenia pole pod wykresem v(t); v rośnie liniowo. */
        let d_position = d_time * self.velocity + d_time * d_velocity / 2.0;

        self.velocity += d_velocity;
        self.position += d_position;

        /* To są współrzędne pudełka ograniczającego ruch cząsteczek.
         * To pewnie nie powinno być zahardcodowane, ale niech na razie
         * tak zostanie. */
        struct Limits {
            min: Vect,
            max: Vect,
        }
        const LIMITS: Limits = Limits {
            min: Vect::new(0.0, 0.0),
            max: Vect::new(1.0, 1.0),
        };

        if self.position.x < LIMITS.min.x {
            self.position.x = LIMITS.min.x;
            self.velocity.x *= -1.0;
        }

        if self.position.x > LIMITS.max.x {
            self.position.x = LIMITS.max.x;
            self.velocity.x *= -1.0;
        }
        if self.position.y < LIMITS.min.y {
            self.position.y = LIMITS.min.y;
            self.velocity.y *= -1.0;
        }
        if self.position.y > LIMITS.max.y {
            self.position.y = LIMITS.max.y;
            self.velocity.y *= -1.0;
        }
    }
}
