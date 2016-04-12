#![allow(dead_code)]

use storage::storage_helper::GitAccess;

pub enum PermissionLevel {
    Editor,
    Viewer
}

pub fn get_permission_level(ga: &GitAccess) -> PermissionLevel {
    return match ga.push() {
        Ok(()) => PermissionLevel::Editor,
        Err(_) => PermissionLevel::Viewer,
    };
}
