// Engine library root
// This file declares the modules for the engine crate.

pub mod config;
pub mod data;
pub mod indicators;
pub mod services;
pub mod models; // Even if models/candle.rs is minimal, the module itself exists.

// You might also include common error types or utility functions specific to the engine library here.
// For example:
// pub mod error;

// The build script will place generated protobuf code in src/services/generated,
// which is then included by src/services/mod.rs.

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // Basic test for the library crate
        assert_eq!(2 + 2, 4);
    }
}
