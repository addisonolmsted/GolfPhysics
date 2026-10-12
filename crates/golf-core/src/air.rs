//! Air density and viscosity, for drag, lift and the Reynolds number.

use crate::math;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Air {
    pub density_kgpm3: f64,
    /// dynamic viscosity, Pa s
    pub viscosity_pas: f64,
}

impl Air {
    /// From temperature, station pressure and relative humidity. Moist air as the sum of dry-air and vapour
    /// partial densities; saturation pressure by the Magnus formula; viscosity by Sutherland's law.
    pub fn new(temp_c: f64, pressure_pa: f64, rel_humidity: f64) -> Air {
        let t = temp_c + 273.15;
        let p_sat = 610.94 * math::exp(17.625 * temp_c / (temp_c + 243.04));
        let p_v = rel_humidity.clamp(0.0, 1.0) * p_sat;
        let rho = (pressure_pa - p_v) / (287.058 * t) + p_v / (461.495 * t);
        let mu = 1.458e-6 * t * t.sqrt() / (t + 110.4);
        Air { density_kgpm3: rho, viscosity_pas: mu }
    }

    /// station pressure at an altitude (standard atmosphere)
    pub fn at_altitude(temp_c: f64, altitude_m: f64, rel_humidity: f64) -> Air {
        let p = 101_325.0 * math::pow(1.0 - 2.25577e-5 * altitude_m, 5.25588);
        Air::new(temp_c, p, rel_humidity)
    }

    /// TrackMan's normalised conditions: sea level, 75 F, 75 % humidity (data/calibration/reference.v1.json)
    pub fn trackman_normal() -> Air {
        Air::new((75.0 - 32.0) / 1.8, 101_325.0, 0.75)
    }

    pub fn kinematic_viscosity(&self) -> f64 {
        self.viscosity_pas / self.density_kgpm3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn densities_are_sane() {
        let a = Air::new(15.0, 101_325.0, 0.0);
        assert!((a.density_kgpm3 - 1.225).abs() < 0.002);
        let tm = Air::trackman_normal();
        assert!(tm.density_kgpm3 > 1.17 && tm.density_kgpm3 < 1.19, "{}", tm.density_kgpm3);
        assert!(Air::at_altitude(15.0, 1600.0, 0.0).density_kgpm3 < 1.05);
    }
}
