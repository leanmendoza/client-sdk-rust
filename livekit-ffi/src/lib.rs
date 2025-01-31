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

use lazy_static::lazy_static;
use livekit::prelude::*;
use prost::Message;
use server::FfiDataBuffer;
use std::{borrow::Cow, sync::Arc};
use thiserror::Error;

mod conversion;
mod proto;
mod server;

#[derive(Error, Debug)]
pub enum FfiError {
    #[error("the server is not configured")]
    NotConfigured,
    #[error("the server is already initialized")]
    AlreadyInitialized,
    #[error("room error {0}")]
    Room(#[from] RoomError),
    #[error("invalid request: {0}")]
    InvalidRequest(Cow<'static, str>),
}

/// # SAFTEY: The "C" callback must be threadsafe and not block
pub type FfiCallbackFn = unsafe extern "C" fn(*const u8, usize);
pub type FfiResult<T> = Result<T, FfiError>;
pub type FfiHandleId = u64;

pub const INVALID_HANDLE: FfiHandleId = 0;

lazy_static! {
    pub static ref FFI_SERVER: server::FfiServer = server::FfiServer::default();
}

/// # Safety
///
/// The foreign language must only provide valid pointers
#[no_mangle]
pub unsafe extern "C" fn livekit_ffi_request(
    data: *const u8,
    len: usize,
    res_ptr: *mut *const u8,
    res_len: *mut usize,
) -> FfiHandleId {
    let data = unsafe { std::slice::from_raw_parts(data, len) };
    let res = match proto::FfiRequest::decode(data) {
        Ok(res) => res,
        Err(err) => {
            log::error!("failed to decode request: {}", err);
            return INVALID_HANDLE;
        }
    };

    let res = match server::requests::handle_request(&FFI_SERVER, res) {
        Ok(res) => res,
        Err(err) => {
            log::error!("failed to handle request: {}", err);
            return INVALID_HANDLE;
        }
    }
    .encode_to_vec();

    unsafe {
        *res_ptr = res.as_ptr();
        *res_len = res.len();
    }

    let handle_id = FFI_SERVER.next_id();
    let ffi_data = FfiDataBuffer {
        handle: handle_id,
        data: Arc::new(res),
    };

    FFI_SERVER.store_handle(handle_id, ffi_data);
    handle_id
}

#[no_mangle]
pub extern "C" fn livekit_ffi_drop_handle(handle_id: FfiHandleId) -> bool {
    FFI_SERVER.drop_handle(handle_id)
}
