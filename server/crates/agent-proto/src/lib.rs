// Proto-generated code: suppress workspace lints that don't apply to generated output
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::must_use_candidate,
    clippy::unwrap_used
)]

pub mod ai {
    pub mod agent {
        pub mod platform {
            pub mod v1 {
                tonic::include_proto!("ai.agent.platform.v1");
            }
        }
    }
}

pub use ai::agent::platform::v1::*;
