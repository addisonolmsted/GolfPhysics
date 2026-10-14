//! Face and path through impact and flight: start direction, spin axis, curve and where the ball lands, for the
//! tour delivery of a driver and a 7 iron (docs/03-success-indicators.md §1, the impact table). + = right.

use golf_core::{Air, ClubDelivery, Vec2};
use golf_flight::{AeroParams, Flat, Model, Wind};
use golf_impact::{ClubTable, Lie, impact};

fn main() {
    let table = ClubTable::builtin();
    let aero = AeroParams::builtin();
    let ground = Flat(0.0);
    let m = Model::new(aero, Air::trackman_normal(), Wind::CALM, &ground);
    // (face to target, face to path) in degrees, and what TrackMan's D-plane says should happen
    let cases = [(0.0, 0.0, "straight"), (2.0, 4.0, "push start, fades right"), (-2.0, -4.0, "pull start, draws left"), (0.0, 4.0, "starts straight, fades"), (6.0, 10.0, "slice"), (0.0, 1.0, "1° open to path"), (0.0, -1.0, "1° closed to path")];
    for (id, mph, attack) in [("dr", 113.0, -1.3), ("7i", 90.0, -4.3), ("pw", 83.0, -5.0)] {
        let club = table.get(id).unwrap();
        println!("{} ({mph} mph, tour delivery):", club.name);
        println!("  face  f-to-p  path | start   axis   spin |  carry  side at landing  curve | expected");
        for (face, ftp, what) in cases {
            let mut d = ClubDelivery::square(mph * 0.44704, attack, club.loft_deg + club.tour_lean_deg);
            d.face_rad = f64::to_radians(face);
            d.path_rad = f64::to_radians(face - ftp);
            let i = impact(&table, club, &d, 0.0, &Lie::CLEAN);
            let f = m.fly(Vec2::ZERO, &i.launch);
            let start = -i.launch.direction_rad.to_degrees();
            let carry = f.carry_m() / 0.9144;
            let side = -f.last.p.y / 0.9144;
            // curve: the landing point's offset from the start line
            let straight = carry * start.to_radians().sin();
            println!("  {face:+4.0}  {ftp:+5.0}  {:+4.0} | {start:+5.1}  {:+5.1}  {:5.0} | {carry:6.1}  {side:+7.1}  {:+6.1} | {what}", face - ftp, i.launch.spin_axis_rad.to_degrees(), i.launch.spin_radps * 30.0 / std::f64::consts::PI, side - straight);
        }
    }
}
