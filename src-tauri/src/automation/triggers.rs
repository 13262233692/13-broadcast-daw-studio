use crate::timecode::Timecode;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerCondition {
    AtExactFrame,
    AfterFrame,
    BeforeFrame,
    EveryNFrames { n: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerAction {
    MuteTrack { track_id: Uuid, mute: bool },
    SoloTrack { track_id: Uuid, solo: bool },
    BypassNode { node_id: Uuid, bypass: bool },
    StartDucking,
    StopDucking,
    BroadcastPosition,
    SetTrackGain { track_id: Uuid, value: f32 },
    SetMasterVolume { value: f32 },
    TriggerCue { cue_id: Uuid },
    StartTransport,
    StopTransport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimecodeTrigger {
    pub id: Uuid,
    pub name: String,
    pub condition: TriggerCondition,
    pub threshold: Timecode,
    pub action: TriggerAction,
    pub fired: bool,
    pub rearm_after_reset: bool,
}

impl TimecodeTrigger {
    pub fn new(name: impl Into<String>, condition: TriggerCondition, threshold: Timecode, action: TriggerAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            condition,
            threshold,
            action,
            fired: false,
            rearm_after_reset: true,
        }
    }

    pub fn check(&mut self, current: &Timecode, frames_passed: i64) -> Option<TriggerAction> {
        let cur_frames = current.to_total_frames();
        let thr_frames = self.threshold.to_total_frames();
        let should_fire = match self.condition {
            TriggerCondition::AtExactFrame => cur_frames == thr_frames,
            TriggerCondition::AfterFrame => cur_frames >= thr_frames,
            TriggerCondition::BeforeFrame => cur_frames <= thr_frames,
            TriggerCondition::EveryNFrames { n } => frames_passed % n as i64 == 0,
        };
        if should_fire && (!self.fired || matches!(self.condition, TriggerCondition::EveryNFrames { .. })) {
            self.fired = true;
            Some(self.action.clone())
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        if self.rearm_after_reset {
            self.fired = false;
        }
    }
}
