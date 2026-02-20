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
