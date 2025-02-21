use serde::{Deserialize, Serialize};
use serde_json::Value;
use typed_builder::TypedBuilder;

use crate::decorators::thread::Thread;

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq, TypedBuilder)]
pub struct Transport {
    pub return_route: ReturnRoute,
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_route_thread: Option<Thread>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq)]
pub enum ReturnRoute {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "all")]
    All,
    #[serde(rename = "thread")]
    Thread,
}

/// Parses a String using Serde to retrieve a Transport Decorator if it is present, as this is not possible via AriesMessage. Will error if the string is not parsable by Serde (which should not occur if using a proper DIDComm message) or if the transport decorator is not correctly formatted.
pub fn get_transport_decorator_from_string(
    string: &str,
) -> Result<Option<Transport>, serde_json::Error> {
    let raw_message: Value = serde_json::from_str(string)?;
    println!("raw msg {}", raw_message);
    match &raw_message["~transport"] {
        Value::Object(_) => {
            let transport_decorator: Option<Transport> =
                serde_json::from_value(raw_message["~transport"].clone())?;
            Ok(transport_decorator)
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::misc::test_utils;

    #[test]
    fn test_transport_minimals() {
        // all variant
        let transport = Transport::builder().return_route(ReturnRoute::All).build();
        let expected = json!({
                "return_route": "all"
        });
        test_utils::test_serde(transport, expected);
        // none variant
        let transport = Transport::builder().return_route(ReturnRoute::None).build();
        let expected = json!({
                "return_route": "none"
        });
        test_utils::test_serde(transport, expected);
    }
    #[test]
    fn test_transport_extended() {
        // thread variant
        let thread = Thread::builder().thid("<thread id>".to_string()).build();
        let transport = Transport::builder()
            .return_route(ReturnRoute::Thread)
            .return_route_thread(thread)
            .build();
        let expected = json!({
                "return_route": "thread",
                "return_route_thread": { "thid": "<thread id>" }
        });
        test_utils::test_serde(transport, expected);
    }

    #[test]
    fn test_get_transport_decorator_from_string() {
        let transport = Transport::builder().return_route(ReturnRoute::All).build();
        let input_string = r#"
        {
            "~transport":{
                "return_route": "all"
            }
        }"#;

        assert_eq!(
            get_transport_decorator_from_string(input_string)
                .unwrap()
                .unwrap(),
            transport
        );
    }

    #[test]
    fn test_get_transport_decorator_from_string_no_decorator() {
        let input_string = r#"
        {
            "foo": "bar"
        }"#;

        assert_eq!(
            get_transport_decorator_from_string(input_string).unwrap(),
            None
        );
    }

    #[test]
    fn test_get_transport_decorator_from_string_malphormed_decorator() {
        let input_string = r#"
        {
            "~transport":{
                "foo": "bar"
            }
        }"#;

        assert!(get_transport_decorator_from_string(input_string).is_err());
    }
}
