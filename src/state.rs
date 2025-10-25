use dashmap::DashMap;
use std::sync::Arc;

use crate::room::Room;

#[derive(Clone, Default)]
pub struct ServerState {
    pub rooms: Arc<DashMap<String, Room>>
}

impl ServerState {

    pub fn insert_room(&self, code: String, room: Room) {
        self.rooms.insert(code, room);
    }

    pub fn get_room(&self, code: &str) -> Option<Room> {
        self.rooms.get(code).map(|guard| guard.clone())
    }

    pub fn remove_if_empty(&self, code: &str) {
        if let Some(r) = self.rooms.get(code) {
            if r.len() == 0 {
                drop(r);
                self.rooms.remove(code);
            }
        }
    }

    pub fn list_rooms(&self) -> Vec<(String, usize)> {
        self.rooms 
            .iter()
            .map(|e| (e.key().clone(), e.value().len()))
            .collect()
    }
}