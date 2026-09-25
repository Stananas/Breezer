//! Playback queue with next/previous navigation.
//!
//! Generic over the item type so both API models and UI track cards can be
//! queued (the UI variant carries the already-loaded cover image).

use std::collections::VecDeque;

/// A simple FIFO queue of tracks with a current index.
#[derive(Debug, Clone)]
pub struct PlayQueue<T> {
    tracks: VecDeque<T>,
    index: Option<usize>,
}

impl<T> Default for PlayQueue<T> {
    fn default() -> Self {
        Self { tracks: VecDeque::new(), index: None }
    }
}

impl<T: Clone> PlayQueue<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the whole queue (e.g. search results / playlist) and start at
    /// `start_at` (clamped to the queue bounds).
    pub fn set_tracks(&mut self, tracks: Vec<T>, start_at: usize) {
        self.tracks = tracks.into();
        self.index = Some(start_at.min(self.tracks.len().saturating_sub(1)));
        log::debug!("queue set: {} tracks at {:?}", self.tracks.len(), self.index);
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn current(&self) -> Option<&T> {
        self.index.and_then(|i| self.tracks.get(i))
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<&T> {
        self.index = self.index.map(|i| (i + 1).min(self.tracks.len().saturating_sub(1)));
        self.current()
    }

    /// Jump to a specific index (used by shuffle). Clamped to the queue bounds.
    pub fn jump_to(&mut self, i: usize) -> Option<&T> {
        self.index = Some(i.min(self.tracks.len().saturating_sub(1)));
        self.current()
    }

    pub fn prev(&mut self) -> Option<&T> {
        self.index = self.index.map(|i| i.saturating_sub(1));
        self.current()
    }

    /// All queued items (borrowed).
    pub fn tracks(&self) -> &VecDeque<T> {
        &self.tracks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct T(u64);

    #[test]
    fn nav() {
        let mut q = PlayQueue::new();
        q.set_tracks(vec![T(1), T(2), T(3)], 0);
        assert_eq!(q.current().unwrap().0, 1);
        assert_eq!(q.next().unwrap().0, 2);
        assert_eq!(q.next().unwrap().0, 3);
        assert_eq!(q.next().unwrap().0, 3); // clamped at end
        assert_eq!(q.prev().unwrap().0, 2);
        assert_eq!(q.index(), Some(1));
    }

    #[test]
    fn clamp_start() {
        let mut q = PlayQueue::new();
        q.set_tracks(vec![T(9), T(8)], 99);
        assert_eq!(q.current().unwrap().0, 8);
    }
}