use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use anyhow::Context as AnyhowContext;
use base64::Engine;
use quickjs_wasm_rs::{Context, JSError, Value as JsValue};

use map_std::unstable::MapStdUnstable;
use sf_std::MultiMap;

use super::JsValueDebug;

pub const MODULE_NAME: &[&str] = &["__ffi", "unstable"];

pub fn link<H: MapStdUnstable + 'static>(
    context: &mut Context,
    state: Rc<RefCell<H>>,
) -> anyhow::Result<()> {
    let global_object = context
        .global_object()
        .context("Failed to get global object")?;
    let unstable = super::traverse_object(context, global_object, MODULE_NAME)?;
    link_into!(
        context, state, unstable,
        {
            // debug
            "printDebug": __export_print_debug,
            "print": __export_print,
            // env
            "bytes_to_utf8": __export_bytes_to_utf8,
            "utf8_to_bytes": __export_utf8_to_bytes,
            "bytes_to_base64": __export_bytes_to_base64,
            "base64_to_bytes": __export_base64_to_bytes,
            "record_to_urlencoded": __export_record_to_urlencoded,
            // messages
            "message_exchange": __export_message_exchange,
            // streams
            "stream_read": __export_stream_read,
            "stream_write": __export_stream_write,
            "stream_close": __export_stream_close
        }
    );

    Ok(())
}

fn __export_message_exchange<H: MapStdUnstable + 'static>(
    state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let message = ensure_arguments!("message_exchange" args; 0: str);
    let response = map_std::unstable::handle_message(state, message.as_bytes());

    Ok(context.value_from_str(&response).unwrap())
}

fn __export_stream_read<H: MapStdUnstable + 'static>(
    state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let (handle, buf) = ensure_arguments!("stream_read" args; 0: i32, 1: mut_bytes);

    match state.stream_read(handle as _, buf) {
        Ok(count) => Ok(context.value_from_u64(count as u64).unwrap()),
        Err(err) => Err(JSError::Type(format!("stream_read: {}", err))),
    }
}

fn __export_stream_write<H: MapStdUnstable + 'static>(
    state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let (handle, buf) = ensure_arguments!("stream_write" args; 0: i32, 1: bytes);

    match state.stream_write(handle as _, buf) {
        Ok(count) => Ok(context.value_from_u64(count as u64).unwrap()),
        Err(err) => Err(JSError::Type(format!("stream_write: {}", err))),
    }
}

fn __export_stream_close<H: MapStdUnstable + 'static>(
    state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let handle = ensure_arguments!("stream_close" args; 0: i32);

    match state.stream_close(handle as _) {
        Ok(()) => Ok(context.undefined_value().unwrap()),
        Err(err) => Err(JSError::Type(format!("stream_close: {}", err))),
    }
}

fn __export_print<H: MapStdUnstable + 'static>(
    state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let message = ensure_arguments!("print" args; 0: str);
    state.print(message);

    Ok(context.undefined_value().unwrap())
}

fn __export_print_debug<H: MapStdUnstable + 'static>(
    _state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    use std::fmt::Write;

    let mut buffer = String::new();
    for arg in args {
        write!(&mut buffer, " {:#?}", JsValueDebug(arg)).unwrap();
    }
    tracing::debug!("map: {}", buffer);

    Ok(context.undefined_value().unwrap())
}

fn __export_bytes_to_utf8<H: MapStdUnstable + 'static>(
    _state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let bytes = ensure_arguments!("bytes_to_utf8" args; 0: bytes);

    match std::str::from_utf8(bytes) {
        Err(err) => Err(JSError::Type(format!(
            "Could not decode bytes at UTF-8: {}",
            err
        ))),
        Ok(s) => Ok(context.value_from_str(s).unwrap()),
    }
}

fn __export_utf8_to_bytes<H: MapStdUnstable + 'static>(
    _state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let string = ensure_arguments!("utf8_to_bytes" args; 0: str);

    Ok(context.array_buffer_value(string.as_bytes()).unwrap())
}

fn __export_bytes_to_base64<H: MapStdUnstable + 'static>(
    _state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let bytes = ensure_arguments!("bytes_to_base64" args; 0: bytes);

    let result = base64::engine::general_purpose::STANDARD.encode(bytes);

    Ok(context.value_from_str(&result).unwrap())
}

fn __export_base64_to_bytes<H: MapStdUnstable + 'static>(
    _state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let string = ensure_arguments!("base64_to_bytes" args; 0: str);

    match base64::engine::general_purpose::STANDARD.decode(string) {
        Err(err) => Err(JSError::Type(format!(
            "Could not decode string as base64: {}",
            err
        ))),
        Ok(bytes) => Ok(context.array_buffer_value(&bytes).unwrap()),
    }
}

fn __export_record_to_urlencoded<H: MapStdUnstable + 'static>(
    _state: &mut H,
    context: &Context,
    _this: &JsValue,
    args: &[JsValue],
) -> Result<JsValue, JSError> {
    let value = ensure_arguments!("record_to_urlencoded" args; 0: value);
    let mut properties = value.properties().unwrap();

    let mut multimap = MultiMap::new();
    while let (Ok(Some(key)), Ok(value)) = (properties.next_key(), properties.next_value()) {
        if !value.is_array() {
            return Err(JSError::Type("Values must be string arrays".to_string()));
        }

        let length = value
            .get_property("length")
            .unwrap()
            .try_as_integer()
            .unwrap() as u32;
        for i in 0..length {
            let v = value.get_indexed_property(i).unwrap();
            if !v.is_str() {
                return Err(JSError::Type("Values must be string arrays".to_string()));
            }

            let key = key.as_str().unwrap().to_string();
            let value = v.as_str().unwrap().to_string();
            multimap.entry(key).or_default().push(value);
        }
    }
    let result = sf_std::encode_query(&multimap);

    Ok(context.value_from_str(&result).unwrap())
}
