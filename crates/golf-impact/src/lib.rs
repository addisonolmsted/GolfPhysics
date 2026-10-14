//! Club delivery + club + lie -> launch conditions. See README.md and docs/models/impact.md.
//!
//! An oblique impact of the face on the ball (the D-plane): restitution along the face normal, the ball rolling
//! off the face across it, a fitted spin factor per club, gear effect and strike loss, lie factors and the slope.

pub mod club;
pub mod impact;
pub mod lie;

pub use club::{Club, ClubKind, ClubTable};
pub use impact::{Impact, impact};
pub use lie::{Lie, LieFactors, LieTable};
