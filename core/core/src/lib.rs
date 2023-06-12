use std::{
    ops::DerefMut,
    sync::{Mutex, OnceLock},
};

use bindings::MessageExchangeFfi;

use sf_std::{
    abi::{Ptr, Size},
    unstable::perform::{
        set_perform_output_error_in, set_perform_output_exception_in, set_perform_output_result_in,
    },
};

mod sf_core;
use sf_core::{CoreConfiguration, OneClientCore};

use crate::observability::buffer::{SharedEventBuffer, TracingEventBuffer, VecEventBuffer};

mod bindings;
mod observability;

#[cfg(feature = "core_mock")]
mod mock;

static GLOBAL_STATE: Mutex<Option<OneClientCore>> = Mutex::new(None);
static METRICS_BUFFER: OnceLock<SharedEventBuffer<VecEventBuffer>> = OnceLock::new();
static DEVELOPER_DUMP_BUFFER: OnceLock<SharedEventBuffer<VecEventBuffer>> = OnceLock::new();

// WASI functions which would be automatically called from `_start`, but we need to explicitly call them since we are a lib.
extern "C" {
    fn __wasm_call_ctors();
    fn __wasm_call_dtors();
}

#[no_mangle]
#[export_name = "oneclient_core_setup"]
/// Initializes persistent core state.
///
/// This function must not be called twice without calling teardown in between.
pub extern "C" fn __export_oneclient_core_setup() {
    #[cfg(feature = "core_mock")]
    return mock::__export_oneclient_core_setup();

    // call ctors first
    unsafe { __wasm_call_ctors() };

    // initialize observability
    METRICS_BUFFER
        .set(SharedEventBuffer::new(VecEventBuffer::new()))
        .unwrap();
    DEVELOPER_DUMP_BUFFER
        .set(SharedEventBuffer::new(VecEventBuffer::new()))
        .unwrap();
    observability::init_tracing(
        METRICS_BUFFER.get().unwrap().clone(),
        DEVELOPER_DUMP_BUFFER.get().unwrap().clone(),
    );
    tracing::debug!(target: "@user", "oneclient_core_setup called");

    let mut lock = GLOBAL_STATE.lock().unwrap();
    if lock.is_some() {
        panic!("Already setup");
    }

    // here we panic on error because there is nothing to teardown
    let config = match CoreConfiguration::from_env() {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(
                target: "@user",
                "Failed to load core configuration from environment: {}",
                err
            );
            CoreConfiguration::default()
        }
    };
    lock.replace(OneClientCore::new(config).unwrap());
}

#[no_mangle]
#[export_name = "oneclient_core_teardown"]
/// Tears down persistent core state.
///
/// This function must be called exactly once after calling core setup.
pub extern "C" fn __export_oneclient_core_teardown() {
    #[cfg(feature = "core_mock")]
    return mock::__export_oneclient_core_teardown();

    tracing::debug!(target: "@user", "oneclient_core_teardown called");

    match GLOBAL_STATE.try_lock() {
        Err(_) => {
            let developer_dump = DEVELOPER_DUMP_BUFFER.get().unwrap().lock();
            for event in developer_dump.events() {
                eprintln!("TODO: dump event after panic \"{}\"", event);
            }

            // panic!("Global state lock already locked: perform most likely panicked")
        }
        Ok(lock) if lock.is_none() => panic!("Not setup or already torn down"),
        Ok(mut lock) => {
            let state = lock.take();
            std::mem::drop(state); // just to be explicit, would be dropped implicitly anyway

            // call dtors last
            unsafe { __wasm_call_dtors() };
        }
    }
}

#[no_mangle]
#[export_name = "oneclient_core_perform"]
/// Executes perform.
///
/// Must be called after [__export_oneclient_core_setup] and before [__export_oneclient_core_teardown].
///
/// All information about map to be performed will be retrieved through messages.
pub extern "C" fn __export_oneclient_core_perform() {
    #[cfg(feature = "core_mock")]
    return mock::__export_oneclient_core_perform();

    let mut lock = GLOBAL_STATE.lock().unwrap();
    let state: &mut OneClientCore = lock
        .as_mut()
        .expect("Global state missing: has oneclient_core_setup been called?");

    match state.perform() {
        Ok(Ok(result)) => set_perform_output_result_in(result, MessageExchangeFfi),
        Ok(Err(error)) => set_perform_output_error_in(error, MessageExchangeFfi),
        Err(exception) => set_perform_output_exception_in(exception.into(), MessageExchangeFfi),
    }
}

#[no_mangle]
#[export_name = "oneclient_core_send_metrics"]
/// Periodically sends metrics.
///
/// Must be called after [__export_oneclient_core_setup] and before [__export_oneclient_core_teardown].
///
/// The host should call this export periodically to send batched insights.
pub extern "C" fn __export_oneclient_core_send_metrics() {
    #[cfg(feature = "core_mock")]
    return mock::__export_oneclient_core_send_metrics();

    let mut lock = GLOBAL_STATE.lock().unwrap();
    let state: &mut OneClientCore = lock
        .as_mut()
        .expect("Global state missing: has oneclient_core_setup been called?");

    let result = state.send_metrics(METRICS_BUFFER.get().unwrap().lock().deref_mut());
    if let Err(err) = result {
        // if there is an error here that means the core couldn't send a message
        // to the host
        // TODO: should be call teardown and abort or let the host call teardown?
        tracing::error!(target: "@user", "Send metrics error: {:#}", err);
    }
}

#[no_mangle]
#[export_name = "asyncify_alloc_stack"]
#[cfg(feature = "asyncify")]
pub extern "C" fn __export_oneclient_core_async_init(mut data_ptr: Ptr<Size>, stack_size: Size) {
    // We allocate Size elements to ensure correct alignment, but size is in bytes.
    let len = stack_size / std::mem::size_of::<Size>();

    let mut asyncify_stack = Vec::<Size>::new();
    asyncify_stack.reserve_exact(len);
    asyncify_stack.resize(len, 0);
    // leak the stack so deallocation doesn't happen
    let asyncify_stack = asyncify_stack.leak();

    // part of the data contract is that we write the resulting range to the data struct ourselves
    let stack = asyncify_stack.as_mut_ptr_range();
    unsafe {
        data_ptr.mut_ptr().write(stack.start as Size);
        data_ptr.mut_ptr().offset(1).write(stack.end as Size)
    }
}
