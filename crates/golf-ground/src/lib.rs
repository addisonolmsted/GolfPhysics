//! Bounce, roll, putting, cup capture and rest. See README.md and docs/models/ground.md.
//!
//! `BallModel::play` flies a launch with golf-flight, then bounces (turf tilt, restitution falling with impact speed,
//! grip or Coulomb slip with spin), hops, slides and rolls on the course's height function with slope gravity and
//! rolling resistance (greens from stimp), until rest, the cup, water, a plug, or off the world. Material parameters
//! come from data/materials/physics.v1.json.

pub mod bounce;
pub mod materials;
pub mod play;
pub mod record;
pub mod roll;
pub mod terrain;

pub use materials::{GroundMaterial, GroundMaterials};
pub use play::{BallModel, Outcome, RestState};
pub use record::{Event, EventKind, Phase, Sample};
pub use roll::{Cup, capture_speed};
pub use terrain::{Terrain, TestPlane};
