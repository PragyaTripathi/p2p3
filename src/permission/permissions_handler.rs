#![allow(dead_code)]

use storage::storage_helper::GitAccess;

pub enum PermissionLevel {
    Editor,
    Viewer
}

pub fn get_permission_level(ga: GitAccess) -> PermissionLevel {
    match ga.push() {
        Ok(()) => {}
        Err(_) => return PermissionLevel::Viewer,
    }
    PermissionLevel::Editor
}
