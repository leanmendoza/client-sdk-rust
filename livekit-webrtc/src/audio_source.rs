// Copyright 2023 LiveKit, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::imp::audio_source as imp_as;
use livekit_protocol::enum_dispatch;

#[derive(Default, Debug)]
pub struct AudioSourceOptions {
    pub echo_cancellation: bool,
    pub noise_suppression: bool,
    pub auto_gain_control: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum RtcAudioSource {
    #[cfg(not(target_arch = "wasm32"))]
    Native(native::NativeAudioSource),
}

impl RtcAudioSource {
    enum_dispatch!(
        [Native];
        fn set_audio_options(self: &Self, options: AudioSourceOptions) -> ();
        fn audio_options(self: &Self) -> AudioSourceOptions;
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use super::*;
    use crate::audio_frame::AudioFrame;
    use std::fmt::{Debug, Formatter};

    #[derive(Clone)]
    pub struct NativeAudioSource {
        pub(crate) handle: imp_as::NativeAudioSource,
    }

    impl Debug for NativeAudioSource {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            f.debug_struct("NativeAudioSource").finish()
        }
    }

    impl Default for NativeAudioSource {
        fn default() -> Self {
            Self::new(AudioSourceOptions::default())
        }
    }

    impl NativeAudioSource {
        pub fn new(options: AudioSourceOptions) -> NativeAudioSource {
            Self {
                handle: imp_as::NativeAudioSource::new(options),
            }
        }

        pub fn capture_frame(&self, frame: &AudioFrame) {
            self.handle.capture_frame(frame)
        }

        pub fn set_audio_options(&self, options: AudioSourceOptions) {
            self.handle.set_audio_options(options)
        }

        pub fn audio_options(&self) -> AudioSourceOptions {
            self.handle.audio_options()
        }
    }
}
