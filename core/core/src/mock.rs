//! Mocked behaviour of core to test Host applications
//!
//! Use usecase to request some behaviour for perform:
//! - CORE_PERFORM_PANIC
//! - CORE_PERFORM_TRUE
//! - CORE_PERFORM_INPUT_VALIDATION_ERROR

use sf_std::unstable::{
    exception::{PerformException, PerformExceptionErrorCode},
    perform::{set_perform_output_exception_in, set_perform_output_result_in, PerformInput},
    HostValue,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::bindings::MessageExchangeFfi;

extern "C" {
    fn __wasm_call_ctors();
    fn __wasm_call_dtors();
}

pub fn __export_oneclient_core_setup() {
    unsafe { __wasm_call_ctors() };

    // initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::debug!("mocked oneclient core setup");
}

pub fn __export_oneclient_core_teardown() {
    tracing::debug!("mocked oneclient core teardown");
    unsafe { __wasm_call_dtors() };
}

pub fn __export_oneclient_core_perform() {
    let perform_input = PerformInput::take_in(MessageExchangeFfi).unwrap();

    tracing::debug!("mocked oneclient core perform {}", perform_input.usecase);

    match perform_input.usecase.as_str() {
        "CORE_PERFORM_PANIC" => panic!("Requested panic!"),
        "CORE_PERFORM_TRUE" => {
            set_perform_output_result_in(HostValue::Bool(true), MessageExchangeFfi)
        }
        "CORE_PERFORM_INPUT_VALIDATION_ERROR" => set_perform_output_exception_in(
            PerformException {
                error_code: PerformExceptionErrorCode::InputValidationError,
                message: "Test validation error".to_string(),
            },
            MessageExchangeFfi,
        ),
        _ => panic!("Unknown usecase: {}", perform_input.usecase),
    };
}
