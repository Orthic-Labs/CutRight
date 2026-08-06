//! Deterministic capture event pump used by production-worker replay tests.
//!
//! Audio & scripted controls share sample-positioned virtual time. Decoder
//! lanes remain independent events in `ReplayEventSchedule`.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use super::wav_replay_source::{
    CaptureEvent, CaptureSource, ScriptedCaptureEvent, WavReplaySource,
};
use super::worker_clock::{
    ClockError, ExecutorLane, ReplayEventKind, ReplayEventSchedule, VirtualWorkerClock,
    WorkerClock, WorkerInstant,
};

pub(crate) struct ReplayDriver {
    source: WavReplaySource,
    clock: Arc<VirtualWorkerClock>,
    base: WorkerInstant,
    schedule: ReplayEventSchedule,
    kws_flight: LaneFlight,
    main_asr_flight: LaneFlight,
    pending_capture: Option<CaptureEvent>,
    controls: VecDeque<CaptureEvent>,
    completed_lanes: VecDeque<(ExecutorLane, WorkerInstant)>,
    audio_remainder: VecDeque<f32>,
    eof: bool,
}

impl ReplayDriver {
    pub(crate) fn from_samples(
        sample_rate: u32,
        samples: Vec<f32>,
        scripted: Vec<ScriptedCaptureEvent>,
        clock: Arc<VirtualWorkerClock>,
    ) -> Result<Self, String> {
        let source = WavReplaySource::from_samples(sample_rate, samples, scripted)?;
        let base = clock.now();
        Ok(Self {
            source,
            clock,
            base,
            schedule: ReplayEventSchedule::default(),
            kws_flight: LaneFlight::Idle,
            main_asr_flight: LaneFlight::Idle,
            pending_capture: None,
            controls: VecDeque::new(),
            completed_lanes: VecDeque::new(),
            audio_remainder: VecDeque::new(),
            eof: false,
        })
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.eof
    }

    pub(crate) fn mark_lane_submitted(
        &mut self,
        lane: ExecutorLane,
        submitted_at: WorkerInstant,
    ) -> Result<(), ReplayLaneError> {
        let started_at = self.clock.now().max(submitted_at);
        let flight = self.lane_flight_mut(lane);
        match flight {
            LaneFlight::Idle => {
                *flight = LaneFlight::AwaitingDuration {
                    request_at: submitted_at,
                    started_at,
                    pending_at: None,
                };
            }
            LaneFlight::AwaitingDuration {
                request_at,
                pending_at,
                ..
            }
            | LaneFlight::Scheduled {
                request_at,
                pending_at,
            }
            | LaneFlight::CompletionReady {
                request_at,
                pending_at,
            } => {
                if *request_at != submitted_at {
                    *pending_at = Some(submitted_at);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn supply_lane_duration(
        &mut self,
        lane: ExecutorLane,
        request_at: WorkerInstant,
        measured_decode_duration: Duration,
    ) -> Result<(), ReplayLaneError> {
        let LaneFlight::AwaitingDuration {
            request_at: mut active_request_at,
            started_at,
            mut pending_at,
        } = self.lane_flight(lane)
        else {
            return Err(ReplayLaneError::UnexpectedCompletion(lane, request_at));
        };
        if active_request_at != request_at {
            if pending_at == Some(request_at) {
                // Latest-only replacement can swap the predicted next request
                // after the prior completion released its lane but before the
                // executor dequeues it. The replacement starts at the same
                // virtual release instant; the superseded request never ran.
                active_request_at = request_at;
                pending_at = None;
            } else {
                return Err(ReplayLaneError::UnexpectedCompletion(lane, request_at));
            }
        }
        self.schedule
            .schedule_lane_completion(lane, started_at, measured_decode_duration);
        *self.lane_flight_mut(lane) = LaneFlight::Scheduled {
            request_at: active_request_at,
            pending_at,
        };
        Ok(())
    }

    pub(crate) fn schedule_lane_completion(
        &mut self,
        lane: ExecutorLane,
        submitted_at: WorkerInstant,
        measured_decode_duration: Duration,
    ) -> Result<(), ReplayLaneError> {
        self.mark_lane_submitted(lane, submitted_at)?;
        self.supply_lane_duration(lane, submitted_at, measured_decode_duration)
    }

    pub(crate) fn is_waiting_for_lane_duration(&self) -> bool {
        self.awaiting_duration_lane().is_some()
    }

    pub(crate) fn pop_control(&mut self) -> Option<CaptureEvent> {
        self.controls.pop_front()
    }

    pub(crate) fn lane_completion_ready(
        &self,
        lane: ExecutorLane,
        request_at: WorkerInstant,
    ) -> bool {
        self.completed_lanes.contains(&(lane, request_at))
    }

    pub(crate) fn take_lane_completion(
        &mut self,
        lane: ExecutorLane,
        request_at: WorkerInstant,
    ) -> bool {
        let Some(index) = self
            .completed_lanes
            .iter()
            .position(|&ready| ready == (lane, request_at))
        else {
            return false;
        };
        self.completed_lanes.remove(index);
        let LaneFlight::CompletionReady {
            request_at: active_request_at,
            pending_at,
        } = self.lane_flight(lane)
        else {
            return false;
        };
        if active_request_at != request_at {
            return false;
        }
        *self.lane_flight_mut(lane) = match pending_at {
            Some(next_request_at) => LaneFlight::AwaitingDuration {
                request_at: next_request_at,
                started_at: self.clock.now(),
                pending_at: None,
            },
            None => LaneFlight::Idle,
        };
        true
    }

    /// Advance virtual events until the requested decoder completion becomes
    /// visible. Audio that arrives first is queued for the capture consumer,
    /// preserving event order even when a trigger has already stopped capture.
    pub(crate) fn advance_to_lane_completion(
        &mut self,
        lane: ExecutorLane,
        request_at: WorkerInstant,
    ) -> Result<bool, ClockError> {
        if self.take_lane_completion(lane, request_at) {
            return Ok(true);
        }
        loop {
            let Some(event) = self.next_event()? else {
                return Ok(false);
            };
            match event {
                ReplayDriverEvent::AwaitingLaneDuration(_) => return Ok(false),
                ReplayDriverEvent::LaneCompletion(_, _) => {
                    if self.take_lane_completion(lane, request_at) {
                        return Ok(true);
                    }
                }
                ReplayDriverEvent::Capture(CaptureEvent::Audio(block)) => {
                    self.audio_remainder.extend(block.samples);
                }
                ReplayDriverEvent::Capture(CaptureEvent::Eof { .. }) => {
                    self.eof = true;
                }
                ReplayDriverEvent::Capture(control) => {
                    self.controls.push_back(control);
                }
            }
        }
    }

    pub(crate) fn next_event(&mut self) -> Result<Option<ReplayDriverEvent>, ClockError> {
        self.prime_capture_event();
        if let Some(lane) = self.awaiting_duration_lane() {
            return Ok(Some(ReplayDriverEvent::AwaitingLaneDuration(lane)));
        }
        let Some(event) = self.schedule.pop_next_event(&self.clock)? else {
            return Ok(None);
        };
        Ok(Some(match event.kind {
            ReplayEventKind::LaneCompletion(lane) => {
                let LaneFlight::Scheduled {
                    request_at,
                    pending_at,
                } = self.lane_flight(lane)
                else {
                    unreachable!("scheduled completion must have an active lane")
                };
                *self.lane_flight_mut(lane) = LaneFlight::CompletionReady {
                    request_at,
                    pending_at,
                };
                self.completed_lanes.push_back((lane, request_at));
                ReplayDriverEvent::LaneCompletion(lane, request_at)
            }
            ReplayEventKind::AudioBlock | ReplayEventKind::ScriptedControl => {
                ReplayDriverEvent::Capture(
                    self.pending_capture
                        .take()
                        .expect("capture schedule must hold capture event"),
                )
            }
        }))
    }

    pub(crate) fn read_audio(
        &mut self,
        max_samples: usize,
    ) -> Result<Option<Vec<f32>>, ClockError> {
        if !self.audio_remainder.is_empty() {
            return Ok(Some(self.take_remainder(max_samples)));
        }
        loop {
            let Some(event) = self.next_event()? else {
                return Ok(None);
            };
            match event {
                ReplayDriverEvent::AwaitingLaneDuration(_) => return Ok(None),
                ReplayDriverEvent::LaneCompletion(_, _) => {}
                ReplayDriverEvent::Capture(CaptureEvent::Audio(block)) => {
                    self.audio_remainder.extend(block.samples);
                    return Ok(Some(self.take_remainder(max_samples)));
                }
                ReplayDriverEvent::Capture(CaptureEvent::Eof { .. }) => {
                    self.eof = true;
                    return Ok(None);
                }
                ReplayDriverEvent::Capture(control) => {
                    self.controls.push_back(control);
                }
            }
        }
    }

    fn take_remainder(&mut self, max_samples: usize) -> Vec<f32> {
        let take = max_samples.min(self.audio_remainder.len());
        self.audio_remainder.drain(..take).collect()
    }

    fn awaiting_duration_lane(&self) -> Option<ExecutorLane> {
        [ExecutorLane::Kws, ExecutorLane::MainAsr]
            .into_iter()
            .find(|&lane| {
                matches!(
                    self.lane_flight(lane),
                    LaneFlight::AwaitingDuration { .. } | LaneFlight::CompletionReady { .. }
                )
            })
    }

    fn lane_flight(&self, lane: ExecutorLane) -> LaneFlight {
        match lane {
            ExecutorLane::Kws => self.kws_flight,
            ExecutorLane::MainAsr => self.main_asr_flight,
        }
    }

    fn lane_flight_mut(&mut self, lane: ExecutorLane) -> &mut LaneFlight {
        match lane {
            ExecutorLane::Kws => &mut self.kws_flight,
            ExecutorLane::MainAsr => &mut self.main_asr_flight,
        }
    }

    fn prime_capture_event(&mut self) {
        if self.pending_capture.is_some() || self.eof {
            return;
        }
        let Some(event) = self.source.next_capture_event() else {
            self.eof = true;
            return;
        };
        let at = self.base.saturating_add(sample_duration(
            event.at_sample(),
            self.source.sample_rate(),
        ));
        match &event {
            CaptureEvent::Audio(_) | CaptureEvent::Eof { .. } => {
                self.schedule.set_next_audio(Some(at));
            }
            CaptureEvent::Control { .. }
            | CaptureEvent::CaptureError { .. }
            | CaptureEvent::Disconnect { .. } => {
                self.schedule.set_next_control(Some(at));
            }
        }
        self.pending_capture = Some(event);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LaneFlight {
    Idle,
    AwaitingDuration {
        request_at: WorkerInstant,
        started_at: WorkerInstant,
        pending_at: Option<WorkerInstant>,
    },
    Scheduled {
        request_at: WorkerInstant,
        pending_at: Option<WorkerInstant>,
    },
    CompletionReady {
        request_at: WorkerInstant,
        pending_at: Option<WorkerInstant>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplayLaneError {
    UnexpectedCompletion(ExecutorLane, WorkerInstant),
}

#[derive(Debug, PartialEq)]
pub(crate) enum ReplayDriverEvent {
    Capture(CaptureEvent),
    LaneCompletion(ExecutorLane, WorkerInstant),
    AwaitingLaneDuration(ExecutorLane),
}

fn sample_instant(sample: u64, sample_rate: u32) -> WorkerInstant {
    WorkerInstant::from_duration_since_start(sample_duration(sample, sample_rate))
}

fn sample_duration(sample: u64, sample_rate: u32) -> Duration {
    Duration::from_secs_f64(sample as f64 / sample_rate as f64)
}

#[cfg(test)]
mod tests {
    use super::super::wav_replay_source::ScriptedCaptureEventKind;
    use super::*;

    #[test]
    fn pumps_audio_and_control_without_wall_clock_waits() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let scripted = vec![ScriptedCaptureEvent {
            at_sample: 320,
            kind: ScriptedCaptureEventKind::Control("stop".into()),
        }];
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 640], scripted, Arc::clone(&clock))
                .unwrap();

        assert_eq!(driver.read_audio(320).unwrap().unwrap().len(), 320);
        assert_eq!(clock.now(), sample_instant(0, 16_000));
        assert_eq!(driver.read_audio(320).unwrap().unwrap().len(), 320);
        assert!(
            matches!(driver.pop_control(), Some(CaptureEvent::Control { name, .. }) if name == "stop")
        );
        assert_eq!(clock.now(), sample_instant(320, 16_000));
        assert_eq!(driver.read_audio(320).unwrap(), None);
        assert!(driver.is_eof());
        assert_eq!(clock.now(), sample_instant(640, 16_000));
    }

    #[test]
    fn preserves_capture_block_remainder_across_short_reads() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 320], Vec::new(), clock).unwrap();

        assert_eq!(driver.read_audio(100).unwrap().unwrap().len(), 100);
        assert_eq!(driver.read_audio(100).unwrap().unwrap().len(), 100);
        assert_eq!(driver.read_audio(200).unwrap().unwrap().len(), 120);
        assert_eq!(driver.read_audio(320).unwrap(), None);
    }

    #[test]
    fn lane_completion_is_dispatched_before_later_audio() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 640], Vec::new(), Arc::clone(&clock))
                .unwrap();
        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Audio(_)))
        ));
        driver
            .schedule_lane_completion(ExecutorLane::Kws, clock.now(), Duration::from_millis(10))
            .unwrap();

        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::LaneCompletion(
                ExecutorLane::Kws,
                WorkerInstant::ZERO,
            ))
        );
        assert_eq!(clock.now(), sample_instant(160, 16_000));
        assert!(driver.take_lane_completion(ExecutorLane::Kws, WorkerInstant::ZERO));
        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Audio(_)))
        ));
        assert_eq!(clock.now(), sample_instant(320, 16_000));
    }

    #[test]
    fn successive_rows_continue_monotonic_virtual_time() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut first =
            ReplayDriver::from_samples(16_000, vec![0.0; 320], Vec::new(), Arc::clone(&clock))
                .unwrap();
        assert_eq!(first.read_audio(320).unwrap().unwrap().len(), 320);
        assert_eq!(first.read_audio(320).unwrap(), None);
        let row_two_start = clock.now();

        let mut second =
            ReplayDriver::from_samples(16_000, vec![0.0; 320], Vec::new(), Arc::clone(&clock))
                .unwrap();
        assert_eq!(second.read_audio(320).unwrap().unwrap().len(), 320);
        assert_eq!(clock.now(), row_two_start);
        assert_eq!(second.read_audio(320).unwrap(), None);
        assert_eq!(
            clock.now().duration_since(row_two_start),
            Duration::from_millis(20)
        );
    }

    #[test]
    fn unknown_lane_duration_yields_without_advancing_capture_time() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 640], Vec::new(), Arc::clone(&clock))
                .unwrap();
        let submitted_at = clock.now();
        driver
            .mark_lane_submitted(ExecutorLane::Kws, submitted_at)
            .unwrap();

        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::AwaitingLaneDuration(ExecutorLane::Kws))
        );
        assert_eq!(clock.now(), submitted_at);
        assert_eq!(driver.read_audio(320).unwrap(), None);
        assert_eq!(clock.now(), submitted_at);
        assert!(!driver.is_eof());
    }

    #[test]
    fn capture_events_before_measured_completion_dispatch_first() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let scripted = vec![ScriptedCaptureEvent {
            at_sample: 320,
            kind: ScriptedCaptureEventKind::Control("stop".into()),
        }];
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 640], scripted, Arc::clone(&clock))
                .unwrap();
        let submitted_at = clock.now();
        driver
            .mark_lane_submitted(ExecutorLane::Kws, submitted_at)
            .unwrap();
        driver
            .supply_lane_duration(ExecutorLane::Kws, submitted_at, Duration::from_millis(50))
            .unwrap();

        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Audio(_)))
        ));
        assert_eq!(clock.now(), sample_instant(0, 16_000));
        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Control { name, .. })) if name == "stop"
        ));
        assert_eq!(clock.now(), sample_instant(320, 16_000));
        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Audio(_)))
        ));
        assert_eq!(clock.now(), sample_instant(320, 16_000));
        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Eof {
                at_sample: 640
            }))
        );
        assert_eq!(clock.now(), sample_instant(640, 16_000));
        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::LaneCompletion(
                ExecutorLane::Kws,
                submitted_at,
            ))
        );
        assert_eq!(clock.now(), sample_instant(800, 16_000));
    }

    #[test]
    fn independent_unknown_lanes_hold_events_until_both_durations_arrive() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 640], Vec::new(), Arc::clone(&clock))
                .unwrap();
        let submitted_at = clock.now();
        driver
            .mark_lane_submitted(ExecutorLane::Kws, submitted_at)
            .unwrap();
        driver
            .mark_lane_submitted(ExecutorLane::MainAsr, submitted_at)
            .unwrap();
        driver
            .supply_lane_duration(ExecutorLane::Kws, submitted_at, Duration::from_millis(40))
            .unwrap();

        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::AwaitingLaneDuration(
                ExecutorLane::MainAsr
            ))
        );
        assert_eq!(clock.now(), submitted_at);

        driver
            .supply_lane_duration(
                ExecutorLane::MainAsr,
                submitted_at,
                Duration::from_millis(10),
            )
            .unwrap();
        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Audio(_)))
        ));
        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::LaneCompletion(
                ExecutorLane::MainAsr,
                submitted_at,
            ))
        );
        assert_eq!(clock.now(), sample_instant(160, 16_000));
        assert!(driver.take_lane_completion(ExecutorLane::MainAsr, submitted_at));
        assert!(matches!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Audio(_)))
        ));
        assert_eq!(clock.now(), sample_instant(320, 16_000));
        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::LaneCompletion(
                ExecutorLane::Kws,
                submitted_at,
            ))
        );
        assert_eq!(clock.now(), sample_instant(640, 16_000));
        assert!(driver.take_lane_completion(ExecutorLane::Kws, submitted_at));
        assert_eq!(
            driver.next_event().unwrap(),
            Some(ReplayDriverEvent::Capture(CaptureEvent::Eof {
                at_sample: 640
            }))
        );
    }

    #[test]
    fn lane_state_ignores_duplicate_submit_and_rejects_unmatched_duration() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, Vec::new(), Vec::new(), Arc::clone(&clock)).unwrap();
        let submitted_at = clock.now();

        assert_eq!(
            driver.supply_lane_duration(ExecutorLane::Kws, submitted_at, Duration::ZERO),
            Err(ReplayLaneError::UnexpectedCompletion(
                ExecutorLane::Kws,
                submitted_at,
            ))
        );
        driver
            .mark_lane_submitted(ExecutorLane::Kws, clock.now())
            .unwrap();
        assert_eq!(
            driver.mark_lane_submitted(ExecutorLane::Kws, clock.now()),
            Ok(())
        );
        assert!(driver.is_waiting_for_lane_duration());
    }

    #[test]
    fn lane_completion_can_be_taken_by_lane_without_consuming_other_lane() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 640], Vec::new(), Arc::clone(&clock))
                .unwrap();
        let submitted_at = clock.now();
        driver
            .schedule_lane_completion(ExecutorLane::Kws, submitted_at, Duration::from_millis(5))
            .unwrap();
        driver
            .schedule_lane_completion(
                ExecutorLane::MainAsr,
                submitted_at,
                Duration::from_millis(10),
            )
            .unwrap();

        assert_eq!(driver.read_audio(320).unwrap().unwrap().len(), 320);
        assert_eq!(driver.read_audio(320).unwrap(), None);
        assert!(driver.lane_completion_ready(ExecutorLane::Kws, submitted_at));
        assert!(!driver.lane_completion_ready(ExecutorLane::MainAsr, submitted_at));
        assert!(driver.take_lane_completion(ExecutorLane::Kws, submitted_at));
        assert_eq!(driver.read_audio(320).unwrap(), None);
        assert!(driver.lane_completion_ready(ExecutorLane::MainAsr, submitted_at));
        assert!(driver.take_lane_completion(ExecutorLane::MainAsr, submitted_at));
        assert_eq!(driver.read_audio(320).unwrap().unwrap().len(), 320);
    }

    #[test]
    fn pending_latest_request_blocks_capture_and_starts_after_prior_completion() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 960], Vec::new(), Arc::clone(&clock))
                .unwrap();
        assert_eq!(driver.read_audio(320).unwrap().unwrap().len(), 320);
        let first_at = clock.now();
        driver
            .schedule_lane_completion(ExecutorLane::Kws, first_at, Duration::from_millis(10))
            .unwrap();
        let pending_at = first_at.saturating_add(Duration::from_millis(5));
        driver
            .mark_lane_submitted(ExecutorLane::Kws, pending_at)
            .unwrap();

        assert_eq!(driver.read_audio(320).unwrap(), None);
        assert!(driver.lane_completion_ready(ExecutorLane::Kws, first_at));
        assert!(driver.take_lane_completion(ExecutorLane::Kws, first_at));
        let second_started_at = clock.now();
        driver
            .supply_lane_duration(ExecutorLane::Kws, pending_at, Duration::from_millis(10))
            .unwrap();
        assert_eq!(driver.read_audio(320).unwrap(), None);
        assert_eq!(
            clock.now().duration_since(second_started_at),
            Duration::from_millis(10)
        );
        assert!(driver.lane_completion_ready(ExecutorLane::Kws, pending_at));
        assert!(driver.take_lane_completion(ExecutorLane::Kws, pending_at));
        assert_eq!(driver.read_audio(320).unwrap().unwrap().len(), 320);
    }

    #[test]
    fn released_pending_request_can_be_replaced_before_decode_starts() {
        let clock = Arc::new(VirtualWorkerClock::default());
        let mut driver =
            ReplayDriver::from_samples(16_000, vec![0.0; 960], Vec::new(), Arc::clone(&clock))
                .unwrap();
        let first_at = clock.now();
        driver
            .schedule_lane_completion(ExecutorLane::Kws, first_at, Duration::from_millis(10))
            .unwrap();
        let superseded_at = first_at.saturating_add(Duration::from_millis(5));
        driver
            .mark_lane_submitted(ExecutorLane::Kws, superseded_at)
            .unwrap();
        assert!(driver
            .advance_to_lane_completion(ExecutorLane::Kws, first_at)
            .unwrap());

        let replacement_at = first_at.saturating_add(Duration::from_millis(8));
        driver
            .mark_lane_submitted(ExecutorLane::Kws, replacement_at)
            .unwrap();
        driver
            .supply_lane_duration(
                ExecutorLane::Kws,
                replacement_at,
                Duration::from_millis(10),
            )
            .unwrap();
        assert!(driver
            .advance_to_lane_completion(ExecutorLane::Kws, replacement_at)
            .unwrap());
    }
}
