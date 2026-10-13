//! The pitch into the face (Ground Lab), event by event.
use golf_core::{Air, LaunchConditions, Vec2};
use golf_course::SurfaceClass as C;
use golf_flight::{AeroParams, Wind};
use golf_ground::{BallModel, FnTerrain, GroundMaterials, Terrain};
fn smoothstep(u: f64) -> f64 { let u = u.clamp(0.0, 1.0); u * u * (3.0 - 2.0 * u) }
fn main() {
    let t = FnTerrain { h: |x: f64, _y: f64| 1.2 * smoothstep((x - 10.0) / 0.8), class: |x, _| if x < 10.0 { C::GREEN } else { C::ROUGH } };
    let m = GroundMaterials::builtin();
    let b = BallModel::new(&t, &m, AeroParams::builtin(), Air::trackman_normal(), Wind::CALM, None);
    let o = b.play(Vec2::new(-14.0, 0.0), &LaunchConditions::from_monitor(44.0, 18.0, 0.0, 4500.0, 0.0));
    for e in &o.events {
        let s = o.samples.iter().find(|s| s.t >= e.t).unwrap();
        let g = t.surface_at(e.p.x, 0.0).unwrap();
        println!("t {:5.2} x {:7.2} z {:5.2} (ground {:5.2}, slope {:5.2}) v ({:6.2}, {:6.2}) backspin {:6.0} rpm  {:?}", e.t, e.p.x, e.p.z, g.z, g.dzdx, s.v.x, s.v.z, -s.w.y * 30.0 / 3.14159, e.kind);
    }
}
