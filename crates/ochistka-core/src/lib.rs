pub mod appearance;
pub mod assess;
pub mod cleanup;
pub mod fs;
pub mod hardware;
pub mod heuristic;
pub mod inventory;
pub mod layout;
pub mod network;
pub mod policy;
pub mod processes;
pub mod programs;
pub mod quarantine;
pub mod readonly;
pub mod scan;
pub mod system;
pub mod tweaks;

#[cfg(test)]
mod fake_fs;
