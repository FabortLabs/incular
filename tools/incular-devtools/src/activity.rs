//! Bounded metadata for the DevTools transport; request bodies are never retained.
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    #[default]
    Pending,
    Completed,
    Failed,
    TimedOut,
    Disconnected,
}

impl RequestStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::TimedOut => "Timed out",
            Self::Disconnected => "Disconnected",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RequestActivity {
    pub sequence: u64,
    pub request_id: u64,
    pub method: String,
    pub request_bytes: usize,
    pub response_bytes: usize,
    pub elapsed_us: Option<u64>,
    pub status: RequestStatus,
}

#[derive(Default)]
pub struct ActivityLog {
    sequence: u64,
    entries: VecDeque<RequestActivity>,
}

impl ActivityLog {
    pub const CAPACITY: usize = 256;

    pub fn begin(&mut self, request_id: u64, method: &str, request_bytes: usize) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        if self.entries.len() == Self::CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(RequestActivity {
            sequence: self.sequence,
            request_id,
            method: method.chars().take(96).collect(),
            request_bytes,
            response_bytes: 0,
            elapsed_us: None,
            status: RequestStatus::Pending,
        });
        self.sequence
    }

    pub fn finish(
        &mut self,
        sequence: u64,
        status: RequestStatus,
        elapsed_us: u64,
        response_bytes: usize,
    ) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.sequence == sequence)
            && entry.status == RequestStatus::Pending
            && status != RequestStatus::Pending
        {
            entry.status = status;
            entry.elapsed_us = Some(elapsed_us);
            entry.response_bytes = response_bytes;
        }
    }

    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &RequestActivity> {
        self.entries.iter()
    }

    /// Clear completed rows while preserving requests still awaiting a response.
    pub fn clear_completed(&mut self) {
        self.entries
            .retain(|entry| entry.status == RequestStatus::Pending);
    }
}
