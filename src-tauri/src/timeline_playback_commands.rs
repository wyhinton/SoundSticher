use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::{AppHandle, Emitter};

pub type TimelineId = String;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(tag = "type", rename_all = "camelCase")]
// pub enum TimelineSource {
//     Operation { timeline_id: TimelineId },

//     // Future-proofing
//     AudioFile { file_path: String },

//     LiveInput { device_id: String },
// }

// #[tauri::command]
// pub fn timeline_playback_play(
//     timeline: TimelineInfo,
//     start_seconds: Option<f64>,
//     state: State<'_, Arc<OpPlaybackState>>,
//     app: AppHandle,
//     logging: State<'_, Arc<Mutex<LoggingService>>>,
// ) -> Result<(), String> {
//     match timeline.source {
//         TimelineSource::Operation { timeline_id } => {
//             op_playback_play(timeline_id, start_seconds, state, app, logging)
//         }

//         TimelineSource::AudioFile { .. } => {
//             Err("AudioFile timeline playback not implemented yet".into())
//         }

//         TimelineSource::LiveInput { .. } => {
//             Err("LiveInput timeline playback not implemented yet".into())
//         }
//     }
// }

// Right now all of our playback commands are based around and older workflow before we had the concept of multiple timelines. For example op_playback_pause etc. But we want to replace these with a new set of commands like timeline_playback_pause, which will recieve information about the timeline that's being paused. If the #sym:TimelineSource for that timeline is an operation, then our playback commands will use some of the existing designs we have now, and in the future we will add support for other kinds of timeline sources. So esentially we should create a new timeline_playback_commands.rs file, which will have commands like play_timeline(timeline: TimelineInfo, ...) and inside of that we will match for the case that the timline has a source of type operation. We'll want a TimelineSourceEnum
