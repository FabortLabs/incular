//! Opt-in synchronous timing and frame-local aggregation.

use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    fmt,
};

use crate::time::Instant;

const SLICE_SIZE: usize = 500;
type InstantSink = Box<dyn Fn(&str, u64)>;

/// A synchronous block whose timestamps use the recorder's monotonic clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedBlock {
    pub name: Cow<'static, str>,
    pub start_micros: u64,
    pub end_micros: u64,
}

impl TimedBlock {
    #[must_use]
    pub fn duration_micros(&self) -> u64 {
        self.end_micros.saturating_sub(self.start_micros)
    }
}

/// The total duration and number of blocks recorded with a given name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregatedTimedBlock {
    pub name: Cow<'static, str>,
    pub duration_micros: u64,
    pub count: usize,
}

/// A drained batch of timings, in completion order. Aggregates are computed
/// only if requested.
#[derive(Debug, Default)]
pub struct AggregatedTimings {
    timed_blocks: Vec<TimedBlock>,
    aggregates: OnceCell<Vec<AggregatedTimedBlock>>,
}

impl AggregatedTimings {
    #[must_use]
    pub fn timed_blocks(&self) -> &[TimedBlock] {
        &self.timed_blocks
    }

    #[must_use]
    pub fn aggregated_blocks(&self) -> &[AggregatedTimedBlock] {
        self.aggregates.get_or_init(|| {
            let mut by_name = HashMap::<&str, usize>::new();
            let mut aggregates = Vec::<AggregatedTimedBlock>::new();
            for block in &self.timed_blocks {
                let entry = by_name
                    .entry(block.name.as_ref())
                    .or_insert(aggregates.len());
                if *entry == aggregates.len() {
                    aggregates.push(AggregatedTimedBlock {
                        name: block.name.clone(),
                        duration_micros: 0,
                        count: 0,
                    });
                }
                let result = &mut aggregates[*entry];
                result.duration_micros = result
                    .duration_micros
                    .saturating_add(block.duration_micros());
                result.count += 1;
            }
            aggregates
        })
    }

    /// Returns a zero count and duration when this batch contains no block
    /// with the requested name.
    #[must_use]
    pub fn get_aggregated(&self, name: &str) -> AggregatedTimedBlock {
        self.aggregated_blocks()
            .iter()
            .find(|block| block.name == name)
            .cloned()
            .unwrap_or_else(|| AggregatedTimedBlock {
                name: Cow::Owned(name.to_owned()),
                duration_micros: 0,
                count: 0,
            })
    }
}

/// A synchronous timing operation could not be completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugTimelineError {
    CollectionDisabled,
    NoOpenBlock,
    UnfinishedBlocks(usize),
}

impl fmt::Display for DebugTimelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CollectionDisabled => f.write_str("timeline collection is disabled"),
            Self::NoOpenBlock => f.write_str("there is no open timeline block"),
            Self::UnfinishedBlocks(count) => write!(f, "{count} timeline block(s) are still open"),
        }
    }
}

impl std::error::Error for DebugTimelineError {}

struct PendingBlock {
    id: u64,
    name: Cow<'static, str>,
    start_micros: u64,
}

#[derive(Default)]
struct Capture {
    stack: Vec<PendingBlock>,
    slices: Vec<Vec<TimedBlock>>,
    records: usize,
}

impl Capture {
    fn finish(&mut self, end_micros: u64) -> Result<(), DebugTimelineError> {
        let pending = self.stack.pop().ok_or(DebugTimelineError::NoOpenBlock)?;
        if self
            .slices
            .last()
            .is_none_or(|slice| slice.len() == SLICE_SIZE)
        {
            self.slices.push(Vec::with_capacity(SLICE_SIZE));
        }
        self.slices
            .last_mut()
            .expect("a timing slice is available")
            .push(TimedBlock {
                name: pending.name,
                start_micros: pending.start_micros,
                end_micros,
            });
        self.records += 1;
        Ok(())
    }

    fn clear_records(&mut self) {
        self.slices.clear();
        self.records = 0;
    }

    fn collect(&mut self) -> Result<AggregatedTimings, DebugTimelineError> {
        if !self.stack.is_empty() {
            return Err(DebugTimelineError::UnfinishedBlocks(self.stack.len()));
        }
        let mut timed_blocks = Vec::with_capacity(std::mem::take(&mut self.records));
        for mut slice in self.slices.drain(..) {
            timed_blocks.append(&mut slice);
        }
        Ok(AggregatedTimings {
            timed_blocks,
            aggregates: OnceCell::new(),
        })
    }
}

/// A renderer-independent synchronous timing collector. Each instance is
/// explicitly enabled and uses a monotonic clock. Disabled calls do not
/// allocate capture data or inspect the clock.
///
/// This collector does not own a native tracing session. Applications may
/// route instant events to an independent sink without including them in
/// duration aggregates.
pub struct DebugTimeline {
    epoch: Instant,
    next_block: Cell<u64>,
    capture: RefCell<Option<Capture>>,
    instant_sink: Option<InstantSink>,
}

impl Default for DebugTimeline {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugTimeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
            next_block: Cell::new(0),
            capture: RefCell::new(None),
            instant_sink: None,
        }
    }

    #[must_use]
    pub fn with_instant_sink(mut self, sink: impl Fn(&str, u64) + 'static) -> Self {
        self.instant_sink = Some(Box::new(sink));
        self
    }

    /// Current timestamp in microseconds relative to this recorder's origin.
    #[must_use]
    pub fn now_micros(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn collection_enabled(&self) -> bool {
        self.capture.borrow().is_some()
    }

    /// Changes collection state. An actual transition resets the capture;
    /// setting the existing state does not discard data.
    pub fn set_collection_enabled(&self, enabled: bool) {
        let mut capture = self.capture.borrow_mut();
        if capture.is_some() != enabled {
            *capture = enabled.then(Capture::default);
        }
    }

    /// Drops completed data without interrupting currently open blocks.
    pub fn reset(&self) {
        if let Some(capture) = self.capture.borrow_mut().as_mut() {
            capture.clear_records();
        }
    }

    /// Collects and drains all completed synchronous timings.
    ///
    /// # Errors
    /// Reports disabled collection or unclosed synchronous blocks without
    /// otherwise changing the recorder.
    pub fn collect(&self) -> Result<AggregatedTimings, DebugTimelineError> {
        self.capture
            .borrow_mut()
            .as_mut()
            .ok_or(DebugTimelineError::CollectionDisabled)?
            .collect()
    }

    /// Opens a timed block when collection is enabled. Blocks may nest.
    pub fn start_sync(&self, name: impl Into<Cow<'static, str>>) {
        let _ = self.start(name);
    }

    fn start(&self, name: impl Into<Cow<'static, str>>) -> Option<u64> {
        if !self.collection_enabled() {
            return None;
        }
        // A custom label conversion may call back into the recorder.
        let name = name.into();
        let mut capture = self.capture.borrow_mut();
        let capture = capture.as_mut()?;
        let id = self.next_block.get();
        self.next_block.set(
            id.checked_add(1)
                .expect("debug timeline block IDs exhausted"),
        );
        capture.stack.push(PendingBlock {
            id,
            name,
            start_micros: self.now_micros(),
        });
        Some(id)
    }

    /// Finishes the most recently opened block. No work is done when disabled.
    ///
    /// # Errors
    /// Returns `NoOpenBlock` if collection is enabled but nothing is open.
    pub fn finish_sync(&self) -> Result<(), DebugTimelineError> {
        let mut capture = self.capture.borrow_mut();
        let Some(capture) = capture.as_mut() else {
            return Ok(());
        };
        capture.finish(self.now_micros())
    }

    /// Brackets a synchronous callable and preserves its return value.
    /// A correctly nested block closes even if the callable unwinds.
    pub fn time_sync<R>(&self, name: impl Into<Cow<'static, str>>, run: impl FnOnce() -> R) -> R {
        let _scope = self.start(name).map(|id| ScopeGuard { id, timeline: self });
        run()
    }

    /// Emits a labeled event without recording a zero-duration timed block.
    pub fn instant_sync(&self, name: &str) {
        if let Some(sink) = &self.instant_sink {
            sink(name, self.now_micros());
        }
    }
}

struct ScopeGuard<'a> {
    id: u64,
    timeline: &'a DebugTimeline,
}

impl Drop for ScopeGuard<'_> {
    fn drop(&mut self) {
        let mut capture = self.timeline.capture.borrow_mut();
        if let Some(capture) = capture.as_mut()
            && capture
                .stack
                .last()
                .is_some_and(|block| block.id == self.id)
        {
            let _ = capture.finish(self.timeline.now_micros());
        }
    }
}
