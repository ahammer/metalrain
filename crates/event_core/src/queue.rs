use crate::{EventEnvelope, EventPayload, EventResult, EventSourceTag, GameEvent};
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub event: EventEnvelope,
    pub result: EventResult,
    pub frame_processed: u64,
}

#[derive(Resource)]
pub struct EventQueue {
    incoming: VecDeque<EventEnvelope>,
    next_frame: VecDeque<EventEnvelope>,
    journal: VecDeque<JournalEntry>,
    pub(crate) journal_capacity: usize,
}

impl EventQueue {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            incoming: VecDeque::new(),
            next_frame: VecDeque::new(),
            journal: VecDeque::new(),
            journal_capacity: cap,
        }
    }
    pub fn set_journal_capacity(&mut self, cap: usize) {
        self.journal_capacity = cap;
        if self.journal.len() > cap {
            while self.journal.len() > cap {
                self.journal.pop_front();
            }
        }
    }
    pub fn enqueue(&mut self, ev: EventEnvelope, current_frame: u64) {
        if ev.frame_enqueued == current_frame {
            self.incoming.push_back(ev);
        } else {
            self.next_frame.push_back(ev);
        }
    }
    pub fn drain_for_frame(&mut self, _frame: u64) -> Vec<EventEnvelope> {
        let mut drained = Vec::with_capacity(self.incoming.len());
        while let Some(ev) = self.incoming.pop_front() {
            drained.push(ev);
        }
        drained
    }
    pub fn promote_next_frame(&mut self) {
        while let Some(ev) = self.next_frame.pop_front() {
            self.incoming.push_back(ev);
        }
    }
    pub fn push_journal(&mut self, entry: JournalEntry) {
        if self.journal.len() == self.journal_capacity {
            self.journal.pop_front();
        }
        self.journal.push_back(entry);
    }
    pub fn journal(&self) -> impl DoubleEndedIterator<Item = &JournalEntry> {
        self.journal.iter()
    }
}

impl EventQueue {
    pub fn enqueue_game(&mut self, game: GameEvent, source: EventSourceTag, frame: u64) {
        self.enqueue(
            EventEnvelope::new(EventPayload::Game(game), source, frame),
            frame,
        );
    }
}
