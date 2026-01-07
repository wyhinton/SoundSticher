// Playback timeline - schedules when operations are active
//
// The timeline is NOT a renderer. It's a scheduler that answers:
// "Which operations are audible at time t, and how do they combine?"
//
// At playback time:
// 1. Audio engine asks timeline: what's active right now?
// 2. Each active op is asked to render its contribution
// 3. Contributions are summed

use super::op_source::{BoxedPlayableOp, PlayableOp};
use super::types::{AudioSpec, PlaybackOpId, PlaybackResult, SampleTime};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A timeline event represents an operation scheduled at a specific time range
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Unique identifier for this event
    pub id: PlaybackOpId,

    /// Start time in samples
    pub start: SampleTime,

    /// End time in samples (exclusive)
    pub end: SampleTime,

    /// Gain/volume for this event (0.0 to 1.0+)
    pub gain: f32,

    /// Whether this event is muted
    pub muted: bool,

    /// Whether this event should be soloed
    pub solo: bool,
}

impl TimelineEvent {
    pub fn new(id: PlaybackOpId, start: SampleTime, end: SampleTime) -> Self {
        Self {
            id,
            start,
            end,
            gain: 1.0,
            muted: false,
            solo: false,
        }
    }

    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    /// Check if this event is active at the given time
    pub fn is_active_at(&self, t: SampleTime) -> bool {
        !self.muted && t >= self.start && t < self.end
    }

    /// Get the duration of this event
    pub fn duration(&self) -> SampleTime {
        self.end - self.start
    }

    /// Convert absolute time to time relative to this event's start
    pub fn to_local_time(&self, t: SampleTime) -> SampleTime {
        if t >= self.start {
            t - self.start
        } else {
            SampleTime::new(0)
        }
    }
}

/// Registry that holds the actual playable operations
pub struct OpRegistry {
    ops: HashMap<PlaybackOpId, BoxedPlayableOp>,
    next_id: u64,
}

impl OpRegistry {
    pub fn new() -> Self {
        Self {
            ops: HashMap::new(),
            next_id: 0,
        }
    }

    /// Register a new operation and return its ID
    pub fn register(&mut self, op: BoxedPlayableOp) -> PlaybackOpId {
        let id = PlaybackOpId::new(self.next_id);
        self.next_id += 1;
        self.ops.insert(id, op);
        id
    }

    /// Get a mutable reference to an operation
    pub fn get_mut(&mut self, id: PlaybackOpId) -> Option<&mut BoxedPlayableOp> {
        self.ops.get_mut(&id)
    }

    /// Get an immutable reference to an operation
    pub fn get(&self, id: PlaybackOpId) -> Option<&BoxedPlayableOp> {
        self.ops.get(&id)
    }

    /// Remove an operation
    pub fn remove(&mut self, id: PlaybackOpId) -> Option<BoxedPlayableOp> {
        self.ops.remove(&id)
    }

    /// Check if an operation exists
    pub fn contains(&self, id: PlaybackOpId) -> bool {
        self.ops.contains_key(&id)
    }

    /// Get the number of registered operations
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl Default for OpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The playback timeline - schedules operations and determines what's active
pub struct PlaybackTimeline {
    /// Scheduled events (operation references with timing)
    events: Vec<TimelineEvent>,

    /// Audio specification for this timeline
    spec: AudioSpec,

    /// Whether any events have solo enabled
    has_solo: bool,
}

impl PlaybackTimeline {
    pub fn new(spec: AudioSpec) -> Self {
        Self {
            events: Vec::new(),
            spec,
            has_solo: false,
        }
    }

    /// Get the audio specification
    pub fn spec(&self) -> AudioSpec {
        self.spec
    }

    /// Add an event to the timeline
    pub fn add_event(&mut self, event: TimelineEvent) {
        if event.solo {
            self.has_solo = true;
        }
        self.events.push(event);
        self.sort_events();
    }

    /// Remove an event by ID
    pub fn remove_event(&mut self, id: PlaybackOpId) -> Option<TimelineEvent> {
        if let Some(pos) = self.events.iter().position(|e| e.id == id) {
            let event = self.events.remove(pos);
            self.update_solo_state();
            Some(event)
        } else {
            None
        }
    }

    /// Get all events active at a given time
    pub fn get_active_events(&self, t: SampleTime) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| {
                if e.muted {
                    return false;
                }
                if self.has_solo && !e.solo {
                    return false;
                }
                e.is_active_at(t)
            })
            .collect()
    }

    /// Get all event IDs active at a given time
    pub fn get_active_ids(&self, t: SampleTime) -> Vec<PlaybackOpId> {
        self.get_active_events(t).iter().map(|e| e.id).collect()
    }

    /// Get the total duration of the timeline (end of last event)
    pub fn duration(&self) -> SampleTime {
        self.events
            .iter()
            .map(|e| e.end)
            .max()
            .unwrap_or(SampleTime::new(0))
    }

    /// Check if the timeline is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
        self.has_solo = false;
    }

    /// Get all events (for iteration)
    pub fn events(&self) -> &[TimelineEvent] {
        &self.events
    }

    /// Get a mutable reference to an event
    pub fn get_event_mut(&mut self, id: PlaybackOpId) -> Option<&mut TimelineEvent> {
        self.events.iter_mut().find(|e| e.id == id)
    }

    /// Set mute state for an event
    pub fn set_muted(&mut self, id: PlaybackOpId, muted: bool) {
        if let Some(event) = self.get_event_mut(id) {
            event.muted = muted;
        }
    }

    /// Set solo state for an event
    pub fn set_solo(&mut self, id: PlaybackOpId, solo: bool) {
        if let Some(event) = self.get_event_mut(id) {
            event.solo = solo;
        }
        self.update_solo_state();
    }

    /// Set gain for an event
    pub fn set_gain(&mut self, id: PlaybackOpId, gain: f32) {
        if let Some(event) = self.get_event_mut(id) {
            event.gain = gain;
        }
    }

    fn sort_events(&mut self) {
        self.events.sort_by_key(|e| e.start);
    }

    fn update_solo_state(&mut self) {
        self.has_solo = self.events.iter().any(|e| e.solo);
    }
}

/// Thread-safe wrapper for timeline + registry that can be hot-swapped
pub struct PlaybackGraph {
    pub timeline: RwLock<PlaybackTimeline>,
    pub registry: RwLock<OpRegistry>,
}

impl PlaybackGraph {
    pub fn new(spec: AudioSpec) -> Self {
        Self {
            timeline: RwLock::new(PlaybackTimeline::new(spec)),
            registry: RwLock::new(OpRegistry::new()),
        }
    }

    /// Add an operation to the registry and schedule it on the timeline
    pub fn schedule_op(
        &self,
        op: BoxedPlayableOp,
        start: SampleTime,
        end: SampleTime,
    ) -> PlaybackResult<PlaybackOpId> {
        let id = self.registry.write().unwrap().register(op);
        let event = TimelineEvent::new(id, start, end);
        self.timeline.write().unwrap().add_event(event);
        Ok(id)
    }

    /// Get the total duration of the timeline
    pub fn duration(&self) -> SampleTime {
        self.timeline.read().unwrap().duration()
    }

    /// Check if the timeline is empty
    pub fn is_empty(&self) -> bool {
        self.timeline.read().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::sample::SamplePlayableOp;

    #[test]
    fn test_timeline_event_is_active_at() {
        let event = TimelineEvent::new(
            PlaybackOpId::new(0),
            SampleTime::new(100),
            SampleTime::new(200),
        );

        assert!(!event.is_active_at(SampleTime::new(50)));
        assert!(event.is_active_at(SampleTime::new(100)));
        assert!(event.is_active_at(SampleTime::new(150)));
        assert!(!event.is_active_at(SampleTime::new(200))); // End is exclusive
        assert!(!event.is_active_at(SampleTime::new(250)));
    }

    #[test]
    fn test_timeline_event_muted() {
        let mut event = TimelineEvent::new(
            PlaybackOpId::new(0),
            SampleTime::new(100),
            SampleTime::new(200),
        );

        assert!(event.is_active_at(SampleTime::new(150)));

        event.muted = true;
        assert!(!event.is_active_at(SampleTime::new(150)));
    }

    #[test]
    fn test_timeline_event_to_local_time() {
        let event = TimelineEvent::new(
            PlaybackOpId::new(0),
            SampleTime::new(100),
            SampleTime::new(200),
        );

        assert_eq!(event.to_local_time(SampleTime::new(150)).samples(), 50);
        assert_eq!(event.to_local_time(SampleTime::new(100)).samples(), 0);
        assert_eq!(event.to_local_time(SampleTime::new(50)).samples(), 0); // Before start
    }

    #[test]
    fn test_op_registry() {
        let mut registry = OpRegistry::new();
        let spec = AudioSpec::cd_quality();

        let op1 = Box::new(SamplePlayableOp::new(vec![0.0; 100], spec));
        let op2 = Box::new(SamplePlayableOp::new(vec![0.0; 100], spec));

        let id1 = registry.register(op1);
        let id2 = registry.register(op2);

        assert_ne!(id1, id2);
        assert!(registry.contains(id1));
        assert!(registry.contains(id2));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_playback_timeline_basic() {
        let spec = AudioSpec::cd_quality();
        let mut timeline = PlaybackTimeline::new(spec);

        let event1 = TimelineEvent::new(
            PlaybackOpId::new(0),
            SampleTime::new(0),
            SampleTime::new(100),
        );
        let event2 = TimelineEvent::new(
            PlaybackOpId::new(1),
            SampleTime::new(50),
            SampleTime::new(150),
        );

        timeline.add_event(event1);
        timeline.add_event(event2);

        // At t=25, only event1 is active
        let active = timeline.get_active_ids(SampleTime::new(25));
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], PlaybackOpId::new(0));

        // At t=75, both events are active
        let active = timeline.get_active_ids(SampleTime::new(75));
        assert_eq!(active.len(), 2);

        // At t=125, only event2 is active
        let active = timeline.get_active_ids(SampleTime::new(125));
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], PlaybackOpId::new(1));
    }

    #[test]
    fn test_playback_timeline_duration() {
        let spec = AudioSpec::cd_quality();
        let mut timeline = PlaybackTimeline::new(spec);

        assert_eq!(timeline.duration().samples(), 0);

        timeline.add_event(TimelineEvent::new(
            PlaybackOpId::new(0),
            SampleTime::new(0),
            SampleTime::new(100),
        ));
        assert_eq!(timeline.duration().samples(), 100);

        timeline.add_event(TimelineEvent::new(
            PlaybackOpId::new(1),
            SampleTime::new(50),
            SampleTime::new(200),
        ));
        assert_eq!(timeline.duration().samples(), 200);
    }

    #[test]
    fn test_playback_timeline_solo() {
        let spec = AudioSpec::cd_quality();
        let mut timeline = PlaybackTimeline::new(spec);

        let mut event1 = TimelineEvent::new(
            PlaybackOpId::new(0),
            SampleTime::new(0),
            SampleTime::new(100),
        );
        let event2 = TimelineEvent::new(
            PlaybackOpId::new(1),
            SampleTime::new(0),
            SampleTime::new(100),
        );

        timeline.add_event(event1.clone());
        timeline.add_event(event2);

        // Both active initially
        assert_eq!(timeline.get_active_ids(SampleTime::new(50)).len(), 2);

        // Solo event1
        timeline.set_solo(PlaybackOpId::new(0), true);

        // Only event1 should be active now
        let active = timeline.get_active_ids(SampleTime::new(50));
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], PlaybackOpId::new(0));
    }
}
