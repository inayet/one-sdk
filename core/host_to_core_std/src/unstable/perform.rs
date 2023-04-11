use serde::{Deserialize, Serialize};

use super::{HostValue, MessageExchange, EXCHANGE_MESSAGE};

define_exchange_core_to_host! {
    struct PerformInputRequest {
        kind: "perform-input"
    } -> enum PerformInputResponse {
        Ok {
            /// Url of the profile.
            profile_url: String,
            /// Url of the map (e.g. `file://<path>`).
            map_url: String,
            /// Usecase defined in the profile.
            usecase: String,
            /// Input passed into the map.
            map_input: HostValue,
            /// Variables passed to map.
            map_vars: HostValue,
            /// Secrets used to resolve security values.
            map_secrets: HostValue
        },
        Err {
            error: String
        }
    }
}

define_exchange_core_to_host! {
    struct PerformOutputRequest<'a> {
        kind: "perform-output",
        /// Result or error of the map.
        ///
        /// Only errors defined in the profile are returned here.
        map_result: &'a Result<HostValue, HostValue>
    } -> enum PerformOutputResponse {
        Ok,
        Err { error: String }
    }
}

pub struct PerformInput {
    pub profile_url: String,
    pub map_url: String,
    pub usecase: String,
    pub map_input: HostValue,
    pub map_vars: HostValue,
    pub map_secrets: HostValue,
}
pub fn perform_input() -> PerformInput {
    let response = PerformInputRequest::new()
        .send_json(&EXCHANGE_MESSAGE)
        .unwrap();

    match response {
        PerformInputResponse::Ok {
            profile_url,
            map_url,
            usecase,
            map_input,
            map_vars,
            map_secrets,
        } => PerformInput {
            profile_url,
            map_url,
            usecase,
            map_input,
            map_vars,
            map_secrets,
        },
        PerformInputResponse::Err { error } => panic!("perform-input error: {}", error),
    }
}

pub struct PerformOutput {
    pub map_result: Result<HostValue, HostValue>,
}
pub fn perform_output(output: PerformOutput) {
    let response = PerformOutputRequest::new(&output.map_result)
        .send_json(&EXCHANGE_MESSAGE)
        .unwrap();

    match response {
        PerformOutputResponse::Ok => (),
        PerformOutputResponse::Err { error } => panic!("perform-output error: {}", error),
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::{
        HostValue, PerformInputRequest, PerformInputResponse, PerformOutputRequest,
        PerformOutputResponse,
    };

    #[test]
    fn test_message_in_perform_input() {
        let actual = serde_json::to_value(PerformInputRequest {
            kind: PerformInputRequest::KIND,
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            json!({
                "kind": "perform-input"
            })
        )
    }

    #[test]
    fn test_message_out_perform_input() {
        let actual = json!({
            "kind": "ok",
            "profile_url": "quz",
            "map_url": "foo",
            "usecase": "bar",
            "map_input": true,
            "map_vars": null,
            "map_secrets": {
                "TOKEN": "banana"
            }
        });

        match serde_json::from_value::<PerformInputResponse>(actual).unwrap() {
            PerformInputResponse::Ok {
                profile_url,
                map_url,
                usecase,
                map_input,
                map_vars,
                map_secrets,
            } => {
                assert_eq!(profile_url, "quz");
                assert_eq!(map_url, "foo");
                assert_eq!(usecase, "bar");
                assert_eq!(map_input, HostValue::Bool(true));
                assert_eq!(map_vars, HostValue::None);
                assert_eq!(
                    map_secrets,
                    HostValue::Object(BTreeMap::from([(
                        String::from("TOKEN"),
                        HostValue::String(String::from("banana"))
                    )]))
                );
            }
            PerformInputResponse::Err { .. } => unreachable!(),
        }
    }

    #[test]
    fn test_message_in_perform_output() {
        let actual = serde_json::to_value(PerformOutputRequest {
            kind: PerformOutputRequest::KIND,
            map_result: &Ok(HostValue::String("hello".into())),
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            json!({
                "kind": "perform-output",
                "map_result": {"Ok": "hello"}
            })
        )
    }

    #[test]
    fn test_message_out_perform_output() {
        let actual = json!({
            "kind": "ok"
        });

        match serde_json::from_value::<PerformOutputResponse>(actual).unwrap() {
            PerformOutputResponse::Ok => (),
            PerformOutputResponse::Err { .. } => unreachable!(),
        }
    }
}
