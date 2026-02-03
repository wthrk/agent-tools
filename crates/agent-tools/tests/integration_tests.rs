//! Integration tests for agent-tools CLI
//!
//! Tests are organized by command/feature:
//! - basic: version, help
//! - build: build command
//! - update: update command
//! - skill_list: skill list command
//! - skill_install: skill install command
//! - skill_installed: skill installed command
//! - sync: sync command
//! - link_unlink: link/unlink commands
//! - skill_update: skill update command
//! - skill_remove: skill remove command
//! - skill_diff: skill diff command
//! - status: status command
//! - cleanup: cleanup command
//! - skill_new: skill new command
//! - skill_validate: skill validate command

mod integration {
    pub mod common;

    mod basic;
    mod build;
    mod cleanup;
    mod link_unlink;
    mod skill_diff;
    mod skill_install;
    mod skill_installed;
    mod skill_list;
    mod skill_new;
    mod skill_remove;
    mod skill_update;
    mod skill_validate;
    mod status;
    mod sync;
    mod update;
}
