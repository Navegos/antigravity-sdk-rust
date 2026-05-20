pub mod proto {
    #[allow(
        warnings,
        clippy::all,
        clippy::pedantic,
        clippy::nursery,
        clippy::unwrap_used,
        clippy::expect_used
    )]
    pub mod localharness {
        // Include the prost-generated code
        include!(concat!(env!("OUT_DIR"), "/antigravity.localharness.rs"));
        // Include the pbjson-generated serde implementations
        include!(concat!(
            env!("OUT_DIR"),
            "/antigravity.localharness.serde.rs"
        ));
    }
}

pub mod agent;
pub mod connection;
pub mod conversation;
pub mod hooks;
#[cfg(not(target_arch = "wasm32"))]
pub mod local;
pub mod policy;
pub mod tools;
pub mod triggers;
pub mod types;
