// dbcparser/src/lib.rs

#![doc(
    html_logo_url = "https://iot.bzh/images/defaults/company/512-479-max-transp.png",
    html_favicon_url = "https://iot.bzh/images/defaults/favicon.ico"
)]

#[path = "data.rs"]
pub mod data;

// #[path = "parser.rs"]
// mod parser;
// pub use parser::dbc_from_str;

// -----------------------------------------------------------------------------
// proxy for test functions, onlu enabled with the associated feature
// -----------------------------------------------------------------------------
// #[cfg(any(test, feature = "internal-parser-tests"))]
// pub mod test_api {
//     use super::parser;
//     use nom::IResult;

//     use crate::data::{
//         AttributeDefault, AttributeDefinition, AttributeValue, AttributeValueForObject, ByteOrder,
//         Comment, EnvironmentVariable, EnvironmentVariableData, ExtendedMultiplex, Message,
//         MessageTransmitter, MultiplexIndicator, Node, Signal, SignalExtendedValueTypeList,
//         SignalGroups, SignalType, SignalTypeRef, Symbol, ValDescription, ValueDescription,
//         ValueTable, ValueType, Version,
//     };

//     pub fn attribute_default(s: &str) -> IResult<&str, AttributeDefault> {
//         parser::attribute_default(s)
//     }
//     pub fn attribute_definition(s: &str) -> IResult<&str, AttributeDefinition> {
//         parser::attribute_definition(s)
//     }
//     pub fn attribute_value(s: &str) -> IResult<&str, AttributeValue> {
//         parser::attribute_value(s)
//     }
//     pub fn attribute_value_for_object(s: &str) -> IResult<&str, AttributeValueForObject> {
//         parser::attribute_value_for_object(s)
//     }
//     pub fn byte_order(s: &str) -> IResult<&str, ByteOrder> {
//         parser::byte_order(s)
//     }
//     pub fn c_ident(s: &str) -> IResult<&str, String> {
//         parser::c_ident(s)
//     }
//     pub fn c_ident_vec(s: &str) -> IResult<&str, Vec<String>> {
//         parser::c_ident_vec(s)
//     }
//     pub fn char_string(s: &str) -> IResult<&str, &str> {
//         parser::char_string(s)
//     }
//     pub fn comment(s: &str) -> IResult<&str, Comment> {
//         parser::comment(s)
//     }
//     pub fn environment_variable(s: &str) -> IResult<&str, EnvironmentVariable> {
//         parser::environment_variable(s)
//     }
//     pub fn environment_variable_data(s: &str) -> IResult<&str, EnvironmentVariableData> {
//         parser::environment_variable_data(s)
//     }
//     pub fn extended_multiplex(s: &str) -> IResult<&str, ExtendedMultiplex> {
//         parser::extended_multiplex(s)
//     }
//     pub fn message(s: &str) -> IResult<&str, Message> {
//         parser::message(s)
//     }
//     pub fn message_transmitter(s: &str) -> IResult<&str, MessageTransmitter> {
//         parser::message_transmitter(s)
//     }
//     pub fn multiplexer_indicator(s: &str) -> IResult<&str, MultiplexIndicator> {
//         parser::multiplexer_indicator(s)
//     }
//     pub fn new_symbols(s: &str) -> IResult<&str, Vec<Symbol>> {
//         parser::new_symbols(s)
//     }
//     pub fn node(s: &str) -> IResult<&str, Node> {
//         parser::node(s)
//     }
//     pub fn signal(s: &str) -> IResult<&str, Signal> {
//         parser::signal(s)
//     }
//     pub fn signal_extended_value_type_list(s: &str) -> IResult<&str, SignalExtendedValueTypeList> {
//         parser::signal_extended_value_type_list(s)
//     }
//     pub fn signal_groups(s: &str) -> IResult<&str, SignalGroups> {
//         parser::signal_groups(s)
//     }
//     pub fn signal_type(s: &str) -> IResult<&str, SignalType> {
//         parser::signal_type(s)
//     }
//     pub fn value_description(s: &str) -> IResult<&str, ValDescription> {
//         parser::value_description(s)
//     }
//     pub fn value_descriptions(s: &str) -> IResult<&str, ValueDescription> {
//         parser::value_descriptions(s)
//     }
//     pub fn value_table(s: &str) -> IResult<&str, ValueTable> {
//         parser::value_table(s)
//     }
//     pub fn value_type(s: &str) -> IResult<&str, ValueType> {
//         parser::value_type(s)
//     }
//     pub fn version(s: &str) -> IResult<&str, Version> {
//         parser::version(s)
//     }
//     pub fn signal_type_ref(s: &str) -> IResult<&str, SignalTypeRef> {
//         parser::signal_type_ref(s)
//     }
// }

// gencode + exports
#[path = "gencode.rs"]
pub mod gencode;

//pub use crate::data::*;
pub use crate::gencode::*;

pub mod prelude {
    // pub use crate::data::*;
    pub use crate::gencode::*;
    // ub use crate::parser::dbc_from_str;
}

