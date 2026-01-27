/*
 * Copyright (C) 2015-2026 IoT.bzh Company
 * Author: Fulup Ar Foll <fulup@iot.bzh>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * This source code includes material derived from Apache-2.0 licensed projects:
 *   - https://github.com/technocreatives/dbc-codegen
 *     Copyright: Marcel Buesing, Pascal Hertleif, Andres Vahter, ...
 *   - https://github.com/marcelbuesing/can-dbc
 *     Copyright: Marcel Buesing
 *
 * Reference:
 *   http://mcu.so/Microcontroller/Automotive/dbc-file-format-documentation_compress.pdf
 */

use heck::{ToSnakeCase, ToUpperCamelCase};

use can_dbc::*;
use libc;
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{self, Error, Write};

pub trait SigCodeGen<T> {
    /// Generate code for a signal.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_code_signal(&self, code: T, msg: &Message) -> io::Result<()>;
    /// Generate code to build an "any" CAN frame.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_can_any_frame(&self, code: T, msg: &Message) -> io::Result<()>;
    /// Generate code to build a standard CAN frame.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_can_std_frame(&self, code: T, msg: &Message) -> io::Result<()>;
    //fn gen_can_mux_frame(&self, code: T, msg: &Message) -> io::Result<()>;
    /// Generate the signal trait.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_signal_trait(&self, code: T, msg: &Message) -> io::Result<()>;
    /// Generate min/max helpers from DBC.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_dbc_min_max(&self, code: T, msg: &Message) -> io::Result<()>;

    /// Generate signal impl.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_signal_impl(&self, code: T, msg: &Message) -> io::Result<()>;
    /// Generate signal enum.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_signal_enum(&self, code: T, msg: &Message) -> io::Result<()>;
}

pub trait MsgCodeGen<T> {
    /// Generate the code for one message.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_code_message(&self, code: T) -> io::Result<()>;
    /// Generate the CAN/DBC message definition.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_can_dbc_message(&self, code: T) -> io::Result<()>;

    /// Generate the CAN/DBC impl section.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn gen_can_dbc_impl(&self, code: T) -> io::Result<()>;
}

pub trait ValCodeGen {
    fn get_type_kamel(&self) -> String;
    fn get_data_value(&self, _data: &str) -> String {
        "no-value".to_string()
    }
}

pub trait SignalCodeGen {
    fn le_start_end_bit(&self, msg: &Message) -> io::Result<(u64, u64)>;
    fn be_start_end_bit(&self, msg: &Message) -> io::Result<(u64, u64)>;
    fn get_data_usize(&self) -> String;
    fn get_data_isize(&self) -> String;
    fn has_scaling(&self) -> bool;
    fn get_data_type(&self) -> String;
    fn get_type_kamel(&self) -> String;
    fn get_type_snake(&self) -> String;
}

pub struct DbcCodeGen {
    outfd: Option<File>,
    dbcfd: Dbc,
    range_check: bool,
    serde_json: bool,
}

pub struct DbcParser {
    uid: &'static str,
    infile: Option<String>,
    outfile: Option<String>,
    range_check: bool,
    serde_json: bool,
    header: Option<&'static str>,
    whitelist: Option<Vec<u32>>,
    blacklist: Option<Vec<u32>>,
}

const KEYWORDS: [&str; 53] = [
    // https://doc.rust-lang.org/stable/reference/keywords.html
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "union",
    // Internal names
    "_other",
];

macro_rules! code_output {
    ($code:expr, $text:expr $(,)?) => {
        $code.output("", $text)
    };
    ($code:expr, $fmt:expr, $($args:tt)+) => {
        $code.output("", format!($fmt, $($args)+))
    };
}

fn get_ctime(format: &str) -> io::Result<String> {
    let fmt = CString::new(format)
        .map_err(|_| io::Error::other("invalid format string (CString::new)"))?;

    // SAFETY: libc::time(NULL) returns current time or -1 on error.
    let t = unsafe { libc::time(std::ptr::null_mut()) };
    if t == -1 {
        return Err(io::Error::last_os_error());
    }

    let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();

    // SAFETY:
    // - &t is a valid pointer to time_t
    // - tm.as_mut_ptr() is valid for writes of libc::tm
    // - if localtime_r returns non-null, tm is initialized
    let tm_ptr = unsafe { libc::localtime_r(&t as *const libc::time_t, tm.as_mut_ptr()) };
    if tm_ptr.is_null() {
        return Err(io::Error::last_os_error());
    }

    let tm = unsafe { tm.assume_init() };

    let mut buf = [0u8; 128];

    // SAFETY:
    // - buf is valid for writes of buf.len()
    // - fmt is a valid NUL-terminated C string
    // - &tm points to an initialized libc::tm
    let n = unsafe {
        libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            fmt.as_ptr(),
            &tm as *const libc::tm,
        )
    };

    if n == 0 {
        return Err(io::Error::other("strftime() returned 0"));
    }

    Ok(String::from_utf8_lossy(&buf[..n]).into_owned())
}

/// Returns current time formatted with `format`.
///
/// # Errors
/// Returns an I/O error if time formatting fails or the system clock is unavailable.
pub fn get_time(format: &str) -> Result<String, Error> {
    get_ctime(format).map_err(|e| Error::other(format!("get_ctime failed: {e}")))
}

fn is_keyword(ident: &str) -> bool {
    KEYWORDS.iter().any(|kw| kw.eq_ignore_ascii_case(ident))
}

fn needs_prefix(ident: &str) -> bool {
    is_keyword(ident) || !ident.starts_with(|c: char| c.is_ascii_alphabetic())
}

impl ValCodeGen for ValDescription {
    fn get_type_kamel(&self) -> String {
        if needs_prefix(&self.description) {
            format!("X{}", self.description).to_upper_camel_case()
        } else {
            // to_upper_camel_case() takes &self; no clone/owned needed
            self.description.to_upper_camel_case()
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn get_data_value(&self, data: &str) -> String {
        match data {
            "bool" => ((self.id as i64) == 1).to_string(),
            "f64" => format!("{}_f64", self.id),
            _ => format!("{}_{}", self.id as i64, data),
        }
    }
}

impl ValCodeGen for Message {
    fn get_type_kamel(&self) -> String {
        if needs_prefix(&self.name) {
            format!("X{}", self.name).to_upper_camel_case()
        } else {
            self.name.to_upper_camel_case()
        }
    }
}

impl SignalCodeGen for Signal {
    fn le_start_end_bit(&self, msg: &Message) -> io::Result<(u64, u64)> {
        let msg_bits = msg.size.checked_mul(8).ok_or_else(|| {
            Error::other(format!(
                "message:{} size overflow while computing bits (size:{} bytes)",
                msg.name, msg.size
            ))
        })?;

        let start_bit = self.start_bit;
        let end_bit = self
            .start_bit
            .checked_add(self.size)
            .ok_or_else(|| Error::other(format!("signal:{} end_bit overflow", self.name)))?;

        if start_bit >= msg_bits {
            return Err(Error::other(format!(
                "signal:{} starts at {}, but message is only {} bits",
                self.name, start_bit, msg_bits
            )));
        }

        if end_bit > msg_bits {
            return Err(Error::other(format!(
                "signal:{} ends at {}, but message is only {} bits",
                self.name, end_bit, msg_bits
            )));
        }

        Ok((start_bit, end_bit))
    }

    fn be_start_end_bit(&self, msg: &Message) -> io::Result<(u64, u64)> {
        let msg_bits = msg.size.checked_mul(8).ok_or_else(|| {
            Error::other(format!(
                "message:{} size overflow while computing bits (size:{} bytes)",
                msg.name, msg.size
            ))
        })?;

        let byte_base = self
            .start_bit
            .checked_div(8)
            .and_then(|v| v.checked_mul(8))
            .ok_or_else(|| Error::other(format!("signal:{} start_bit overflow", self.name)))?;

        let bit_in_byte = self
            .start_bit
            .checked_rem(8)
            .ok_or_else(|| Error::other(format!("signal:{} start_bit overflow", self.name)))?;

        let bit_from_msb = 7u64
            .checked_sub(bit_in_byte)
            .ok_or_else(|| Error::other(format!("signal:{} start_bit overflow", self.name)))?;

        let start_bit = byte_base
            .checked_add(bit_from_msb)
            .ok_or_else(|| Error::other(format!("signal:{} start_bit overflow", self.name)))?;

        let end_bit = start_bit
            .checked_add(self.size)
            .ok_or_else(|| Error::other(format!("signal:{} end_bit overflow", self.name)))?;

        if start_bit > msg_bits {
            return Err(Error::other(format!(
                "signal:{} starts at {}, but message is only {} bits",
                self.name, start_bit, msg_bits
            )));
        }

        if end_bit > msg_bits {
            return Err(Error::other(format!(
                "signal:{} ends at {}, but message is only {} bits",
                self.name, end_bit, msg_bits
            )));
        }

        Ok((start_bit, end_bit))
    }

    fn get_data_usize(&self) -> String {
        let size = match self.size {
            n if n <= 8 => "u8",
            n if n <= 16 => "u16",
            n if n <= 32 => "u32",
            _ => "u64",
        };
        size.to_string()
    }

    fn get_data_isize(&self) -> String {
        let size = match self.size {
            n if n <= 8 => "i8",
            n if n <= 16 => "i16",
            n if n <= 32 => "i32",
            _ => "i64",
        };
        size.to_string()
    }

    #[inline]
    fn has_scaling(&self) -> bool {
        const EPS: f64 = 1e-12;
        self.offset.abs() > EPS || (self.factor - 1.0).abs() > EPS
    }

    fn get_data_type(&self) -> String {
        if self.size == 1 {
            "bool".into()
        } else if self.has_scaling() {
            "f64".into()
        } else {
            let size = match self.size {
                n if n <= 8 => "8",
                n if n <= 16 => "16",
                n if n <= 32 => "32",
                _ => "64",
            };
            match self.value_type {
                ValueType::Signed => format!("i{size}"),
                ValueType::Unsigned => format!("u{size}"),
            }
        }
    }

    fn get_type_kamel(&self) -> String {
        if needs_prefix(&self.name) {
            format!("X{}", self.name).to_upper_camel_case()
        } else {
            self.name.to_upper_camel_case()
        }
    }

    fn get_type_snake(&self) -> String {
        if needs_prefix(&self.name) {
            format!("X{}", self.name).to_snake_case()
        } else {
            self.name.to_snake_case()
        }
    }
}

impl SigCodeGen<&DbcCodeGen> for Signal {
    #[allow(clippy::too_many_lines)]
    fn gen_signal_trait(&self, code: &DbcCodeGen, msg: &Message) -> io::Result<()> {
        let msg_type = msg.get_type_kamel();
        let sig_type = self.get_type_kamel();

        let read_fn = match self.byte_order {
            ByteOrder::LittleEndian => {
                let (start_bit, end_bit) = self.le_start_end_bit(msg)?;

                format!(
                    "frame.data.view_bits::<Lsb0>()[{start}..{end}].load_le::<{typ}>()",
                    typ = self.get_data_usize(),
                    start = start_bit,
                    end = end_bit,
                )
            },
            ByteOrder::BigEndian => {
                let (start_bit, end_bit) = self.be_start_end_bit(msg)?;

                format!(
                    "frame.data.view_bits::<Msb0>()[{start}..{end}].load_be::<{typ}>()",
                    typ = self.get_data_usize(),
                    start = start_bit,
                    end = end_bit
                )
            },
        };

        code_output!(
            code,
            format!(
                r#"    /// {msg_type}::{sig_type} public api (CanDbcSignal trait)
    impl CanDbcSignal for {sig_type} {{

        fn get_name(&self) -> &'static str {{
            self.name
        }}

        fn get_stamp(&self) -> u64 {{
            self.stamp
        }}

        fn get_status(&self) -> CanDataStatus{{
            self.status
        }}

        fn as_any(&mut self) -> &mut dyn Any {{
            self
        }}

        fn update(&mut self, frame: &CanMsgData) -> i32 {{
            match frame.opcode {{
                CanBcmOpCode::RxChanged => {{
                    let value = {read_fn};"#
            )
        )?;

        if self.value_type == ValueType::Signed {
            let data_isize = self.get_data_isize();
            code_output!(
                code,
                format!(
                    r#"                    let value = {data_isize}::from_ne_bytes(value.to_ne_bytes());"#
                )
            )?;
        }

        if self.size == 1 {
            code_output!(code, "                    self.value= value == 1;")?;
        } else if self.has_scaling() {
            let offset = self.offset;
            let factor = self.factor;
            code_output!(
                code,
                format!(
                    r#"                    let factor = {factor}_f64;
                    let offset = {offset}_f64;
                    let newval= (value as f64) * factor + offset;
                    if newval != self.value {{
                        self.value= newval;
                        self.status= CanDataStatus::Updated;
                        self.stamp= frame.stamp;
                    }} else {{
                        self.status= CanDataStatus::Unchanged;
                    }}"#
                )
            )?;
        } else {
            code_output!(
                code,
                r#"                    if self.value != value {
                        self.value= value;
                        self.status= CanDataStatus::Updated;
                        self.stamp= frame.stamp;
                    } else {
                        self.status= CanDataStatus::Unchanged;
                    }"#
            )?;
        }

        let data_type = self.get_data_type();
        let dtype_enum = data_type.as_str().to_upper_camel_case();

        code_output!(
            code,
            format!(
                r#"                }},
                CanBcmOpCode::RxTimeout => {{
                    self.status=CanDataStatus::Timeout;
                }},
                _ => {{
                    self.status=CanDataStatus::Error;
                }},
            }}
            match &self.callback {{
                None => 0,
                Some(callback) => {{
                    match callback.try_borrow() {{
                        Err(_) => {{println!("fail to get signal callback reference"); -1}},
                        Ok(cb_ref) => cb_ref.sig_notification(self),
                    }}
                }}
            }}
        }}

        fn set_value(&mut self, value:CanDbcType, data:&mut [u8]) -> Result<(),CanError> {{
            let value:{data_type}= match value.cast() {{
                Ok(val) => val,
                Err(error) => return Err(error)
            }};
            self.set_typed_value(value, data)
        }}

        fn get_value(&self) -> CanDbcType {{
            CanDbcType::{dtype_enum}(self.get_typed_value())
        }}
"#
            )
        )?;

        if code.serde_json {
            code_output!(
                code,
                r#"        fn to_json(&self) -> String {
            match serde_json::to_string(self) {
                Ok(json)=> json,
                _ => "serde-json-error".to_owned()
            }
        }
"#
            )?;
        }

        // reset signal values + set signal notification callback + impl footer
        code_output!(
            code,
            format!(
                r#"        fn reset(&mut self) {{
            self.stamp=0;
            self.reset_value();
            self.status=CanDataStatus::Unset;
        }}

        fn set_callback(&mut self, callback: Box<dyn CanSigCtrl>)  {{
            self.callback= Some(RefCell::new(callback));
        }}

    }} // end {msg_type}::{sig_type} public api
"#
            )
        )?;

        Ok(())
    }

    fn gen_dbc_min_max(&self, code: &DbcCodeGen, _msg: &Message) -> io::Result<()> {
        if self.size == 1 {
            return Ok(());
        }

        let typ = self.get_data_type();
        let name_uc = self.get_type_kamel().to_uppercase();
        let min = self.min;
        let max = self.max;

        code_output!(
            code,
            format!(
                r#"        pub const {name_uc}_MIN: {typ} = {min}_{typ};
        pub const {name_uc}_MAX: {typ} = {max}_{typ};
"#
            )
        )?;
        Ok(())
    }

    fn gen_signal_enum(&self, code: &DbcCodeGen, msg: &Message) -> io::Result<()> {
        if let Some(variants) = code.dbcfd.value_descriptions_for_signal(msg.id, self.name.as_str())
        {
            let id = msg.id.raw();
            let name = self.name.as_str();
            let type_kamel = self.get_type_kamel();
            code_output!(code, format!(r#"    // DBC definition for MsgID:{id} Signal:{name}"#))?;
            if code.serde_json {
                code_output!(code, r#"    #[derive(Serialize, Deserialize)]"#)?;
            }
            code_output!(code, format!(r#"    pub enum Dbc{type_kamel} {{"#))?;
            for variant in variants {
                let variant_name = variant.get_type_kamel();
                code_output!(code, format!(r#"        {variant_name},"#))?;
            }

            let data_type = self.get_data_type();
            code_output!(
                code,
                format!(
                    r#"        _Other({data_type}),
    }}

    impl From<Dbc{type_kamel}> for {data_type} {{
        fn from (val: Dbc{type_kamel}) -> {data_type} {{
            match val {{"#
                )
            )?;
            for variant in variants {
                let type_kamel = self.get_type_kamel();
                let variant_type_kamel = variant.get_type_kamel();
                let variant_data_type = variant.get_data_value(&self.get_data_type());
                code_output!(
                    code,
                    format!(
                        r#"                Dbc{type_kamel}::{variant_type_kamel} => {variant_data_type},"#
                    )
                )?;
            }
            let type_kamel = self.get_type_kamel();
            code_output!(
                code,
                format!(
                    r#"                Dbc{type_kamel}::_Other(x) => x
            }}
        }}
    }}
"#
                )
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn gen_signal_impl(&self, code: &DbcCodeGen, msg: &Message) -> io::Result<()> {
        // signal comments and metadata
        let msg_type_kamel = msg.get_type_kamel();
        let min = self.min;
        let max = self.max;
        let unit = self.unit.as_str();
        let receivers = self.receivers.join(", ");
        let start_bit = self.start_bit;
        let size = self.size;
        let factor = self.factor;
        let offset = self.offset;
        let byte_order = self.byte_order;
        let value_type = self.value_type;

        let type_kamel = self.get_type_kamel();

        let data_type = self.get_data_type();
        let data_usize = self.get_data_usize();

        code_output!(code, format!(r#"    /// {msg_type_kamel}::{type_kamel}"#))?;
        if let Some(comment) = code.dbcfd.signal_comment(msg.id, self.name.as_str()) {
            code_output!(code, r#"    ///"#)?;

            for line in comment.trim().lines() {
                code_output!(code, format!(r#"    /// {line}"#))?;
            }
        }

        code_output!(
            code,
            format!(
                r#"    /// - Min: {min}
    /// - Max: {max}
    /// - Unit: {unit:?}
    /// - Receivers: {receivers}
    /// - Start bit: {start_bit}
    /// - Signal size: {size} bits
    /// - Factor: {factor}
    /// - Offset: {offset}
    /// - Byte order: {byte_order:?}
    /// - Value type: {value_type:?}"#
            )
        )?;

        if code.serde_json {
            code_output!(code, r#"    #[derive(Serialize, Deserialize)]"#)?;
        }
        code_output!(code, format!(r#"    pub struct {type_kamel} {{"#))?;

        if code.serde_json {
            code_output!(code, r#"        #[serde(skip)]"#)?;
        }
        code_output!(
            code,
            format!(
                r#"        callback: Option<RefCell<Box<dyn CanSigCtrl>>>,
        status: CanDataStatus,
        name: &'static str,
        stamp: u64,
        value: {data_type},
    }}
"#
            )
        )?;

        self.gen_signal_enum(code, msg)?;

        // start signal implementation
        code_output!(
            code,
            format!(
                r#"    impl {type_kamel}  {{
        pub fn new() -> Rc<RefCell<Box<dyn CanDbcSignal>>> {{
            Rc::new(RefCell::new(Box::new({type_kamel} {{
                status: CanDataStatus::Unset,
                name:"{type_kamel}","#
            )
        )?;
        if self.size == 1 {
            code_output!(code, r#"                value: false,"#)?;
        } else {
            code_output!(code, format!(r#"                value: 0_{data_type},"#))?;
        }

        code_output!(
            code,
            r#"                stamp: 0,
                callback: None,
            })))
        }

        fn reset_value(&mut self) {"#
        )?;
        if self.size == 1 {
            code_output!(code, r#"            self.value= false;"#)?;
        } else {
            code_output!(code, format!(r#"            self.value= 0_{data_type};"#))?;
        }

        code_output!(
            code,
            r#"        }
"#
        )?;

        if let Some(variants) = code.dbcfd.value_descriptions_for_signal(msg.id, self.name.as_str())
        {
            code_output!(
                code,
                format!(r#"        pub fn get_as_def (&self) -> Dbc{type_kamel} {{"#)
            )?;

            // float is not compatible with match
            if data_type == "f64" {
                code_output!(
                    code,
                    format!(r#"                Dbc{type_kamel}::_Other(self.get_typed_value())"#)
                )?;
            } else {
                let mut count = 0;
                code_output!(code, r#"            match self.get_typed_value() {"#)?;
                for variant in variants {
                    count += 1;

                    let data_value = variant.get_data_value(&data_type);
                    let variant_type_kamel = variant.get_type_kamel();
                    code_output!(
                        code,
                        format!(
                            r#"                {data_value} => Dbc{type_kamel}::{variant_type_kamel},"#
                        )
                    )?;
                }

                // Help in buggy DBC file support
                if count != 2 || self.size != 1 {
                    code_output!(
                        code,
                        format!(
                            r#"                _ => Dbc{type_kamel}::_Other(self.get_typed_value()),"#
                        )
                    )?;
                }
                code_output!(code, r#"            }"#)?;
            }
            code_output!(
                code,
                format!(
                    r#"        }}

        pub fn set_as_def (&mut self, signal_def: Dbc{type_kamel}, data: &mut[u8])-> Result<(),CanError> {{
            match signal_def {{"#
                )
            )?;
            for variant in variants {
                let variant_type_kamel = variant.get_type_kamel();
                let data_value = variant.get_data_value(&data_type);
                code_output!(
                    code,
                    format!(
                        r#"                Dbc{type_kamel}::{variant_type_kamel} => self.set_typed_value({data_value}, data),"#
                    )
                )?;
            }
            code_output!(
                code,
                format!(
                    r#"                Dbc{type_kamel}::_Other(x) => self.set_typed_value(x,data)"#
                )
            )?;
            code_output!(
                code,
                r#"            }
        }"#
            )?;
        }

        code_output!(
            code,
            format!(
                r#"        fn get_typed_value(&self) -> {data_type} {{
            self.value
        }}

        fn set_typed_value(&mut self, value:{data_type}, data:&mut [u8]) -> Result<(),CanError> {{"#
            )
        )?;
        if self.size == 1 {
            code_output!(code, r#"            let value = value as u8;"#)?;
        } else if code.range_check && self.has_scaling() {
            let min = self.min;
            let max = self.max;
            let factor = self.factor;
            let offset = self.offset;
            code_output!(
                code,
                format!(
                    r#"            if value < {min}_{data_type} || {max}_{data_type} < value {{
                return Err(CanError::new("invalid-signal-value",format!("value={{}} not in [{min}..{max}]",value)));
            }}
            let factor = {factor}_f64;
            let offset = {offset}_f64;
            let value = ((value - offset) / factor) as {data_usize};"#
                )
            )?;
        }

        if self.value_type == ValueType::Signed {
            code_output!(
                code,
                format!(
                    r#"            let value = {data_usize}::from_ne_bytes(value.to_ne_bytes());"#
                )
            )?;
        }

        match self.byte_order {
            ByteOrder::LittleEndian => {
                let (start_bit, end_bit) = self.le_start_end_bit(msg)?;
                code_output!(
                    code,
                    format!(
                        r#"            data.view_bits_mut::<Lsb0>()[{start_bit}..{end_bit}].store_le(value);"#
                    )
                )?;
            },
            ByteOrder::BigEndian => {
                let (start_bit, end_bit) = self.be_start_end_bit(msg)?;
                code_output!(
                    code,
                    format!(
                        r#"            data.view_bits_mut::<Msb0>()[{start_bit}..{end_bit}].store_be(value);"#
                    )
                )?;
            },
        }

        let msg_type = msg.get_type_kamel();
        let sig_type = self.get_type_kamel();

        code_output!(
            code,
            format!(
                r#"            Ok(())
        }}

    }} // {msg_type}::{sig_type} impl end
"#
            )
        )?;

        Ok(())
    }

    fn gen_can_std_frame(&self, _code: &DbcCodeGen, _msg: &Message) -> io::Result<()> {
        Ok(())
    }

    fn gen_can_any_frame(&self, code: &DbcCodeGen, msg: &Message) -> io::Result<()> {
        match self.multiplexer_indicator {
            MultiplexIndicator::Plain => self.gen_can_std_frame(code, msg)?,
            MultiplexIndicator::Multiplexor
            | MultiplexIndicator::MultiplexedSignal(_)
            | MultiplexIndicator::MultiplexorAndMultiplexedSignal(_) => {
                // (optional) any shared handling for multiplexed cases
            },
        }
        let sig_type = self.get_type_kamel();

        code_output!(
            code,
            format!(
                r#"    impl fmt::Display for {sig_type} {{
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {{
            let text=format!("{sig_type}:{{}}", self.get_typed_value());
            fmt.pad(&text)
        }}
    }}

    impl fmt::Debug for {sig_type} {{
        fn fmt(&self, format: &mut fmt::Formatter<'_>) -> fmt::Result {{
            format.debug_struct("{sig_type}")
                .field("val", &self.get_typed_value())
                .field("stamp", &self.get_stamp())
                .field("status", &self.get_status())
                .finish()
        }}
    }}
"#
            )
        )?;

        Ok(())
    }

    fn gen_code_signal(&self, code: &DbcCodeGen, msg: &Message) -> io::Result<()> {
        self.gen_signal_impl(code, msg)?;
        self.gen_can_any_frame(code, msg)?;
        self.gen_signal_trait(code, msg)?;
        Ok(())
    }
}

impl MsgCodeGen<&DbcCodeGen> for Message {
    fn gen_can_dbc_impl(&self, code: &DbcCodeGen) -> io::Result<()> {
        let sig_count = self.signals.len();
        let msg_id = self.id.raw();
        let msg_name = self.get_type_kamel();

        code_output!(
            code,
            format!(
                r#"    pub struct DbcMessage {{
        callback: Option<RefCell<Box<dyn CanMsgCtrl>>>,
        signals: [Rc<RefCell<Box<dyn CanDbcSignal>>>;{sig_count}],
        name: &'static str,
        status: CanBcmOpCode,
        listeners: i32,
        stamp: u64,
        id: u32,
    }}

    impl DbcMessage {{
        pub fn new() -> Rc<RefCell<Box <dyn CanDbcMessage>>> {{
            Rc::new(RefCell::new(Box::new (DbcMessage {{
                id: {msg_id},
                name: "{msg_name}",
                status: CanBcmOpCode::Unknown,
                listeners: 0,
                stamp: 0,
                callback: None,
                signals: ["#
            )
        )?;

        for signal in &self.signals {
            let type_id = signal.get_type_kamel();
            code_output!(code, format!(r#"                    {type_id}::new(),"#))?;
        }
        code_output!(
            code,
            r#"                ],
            })))
        }
"#
        )?;

        // set all message signals values
        let args: Vec<String> = self
            .signals
            .iter()
            .map(|signal| format!("{}: {}", signal.get_type_snake(), signal.get_data_type()))
            .collect();

        let args_str = args.join(", ");
        code_output!(
            code,
            format!(
                r#"        pub fn set_values(&mut self, {args_str}, frame: &mut[u8]) -> Result<&mut Self, CanError> {{
"#
            )
        )?;

        for idx in 0..self.signals.len() {
            let dtype_enum = self.signals[idx].get_data_type().to_upper_camel_case();
            let sig_snake = self.signals[idx].get_type_snake();

            code_output!(
                code,
                format!(
                    r#"            match Rc::clone (&self.signals[{idx}]).try_borrow_mut() {{
                Ok(mut signal) => signal.set_value(CanDbcType::{dtype_enum}({sig_snake}), frame)?,
                Err(_) => return Err(CanError::new("signal-set-values-fail","Internal error {sig_snake}:{dtype_enum}")),
            }}"#
                )
            )?;
        }
        code_output!(
            code,
            r#"            Ok(self)
        }
    }
"#
        )?;

        Ok(())
    }

    fn gen_can_dbc_message(&self, code: &DbcCodeGen) -> io::Result<()> {
        // build message signal:type list
        code_output!(
            code,
            r#"    impl CanDbcMessage for DbcMessage {
        fn reset(&mut self) -> Result<(), CanError> {
            self.status=CanBcmOpCode::Unknown;
            self.stamp=0;"#
        )?;

        for idx in 0..self.signals.len() {
            let dtype_enum = self.signals[idx].get_data_type().to_upper_camel_case();
            let sig_snake = self.signals[idx].get_type_snake();

            code_output!(
                code,
                format!(
                    r#"            match Rc::clone (&self.signals[{idx}]).try_borrow_mut() {{
                Ok(mut signal) => signal.reset(),
                Err(_) => return Err(CanError::new("signal-reset-fail","Internal error {sig_snake}:{dtype_enum}")),
            }}"#
                )
            )?;
        }
        code_output!(
            code,
            r#"        Ok(())
    }

        fn update(&mut self, frame: &CanMsgData) -> Result<(), CanError> {
            self.stamp= frame.stamp;
            self.status= frame.opcode;
            self.listeners= 0;"#
        )?;

        for idx in 0..self.signals.len() {
            let dtype_enum = self.signals[idx].get_data_type().to_upper_camel_case();
            let sig_snake = self.signals[idx].get_type_snake();

            code_output!(
                code,
                format!(
                    r#"            match Rc::clone (&self.signals[{idx}]).try_borrow_mut() {{
                Ok(mut signal) => self.listeners += signal.update(frame),
                Err(_) => return Err(CanError::new("signal-update-fail","Internal error {sig_snake}:{dtype_enum}")),
            }}"#
                )
            )?;
        }
        let msg_type = self.get_type_kamel();

        code_output!(
            code,
            format!(
                r#"            match &self.callback {{
                None => {{}},
                Some(callback) => {{
                    match callback.try_borrow() {{
                        Err(_) => println!("fail to get message callback reference"),
                        Ok(cb_ref) => cb_ref.msg_notification(self),
                    }}
                }}
            }}
            Ok(())
        }}

        fn get_signals(&self) -> &[Rc<RefCell<Box<dyn CanDbcSignal>>>] {{
            &self.signals
        }}

        fn get_listeners(&self) -> i32 {{
            self.listeners
        }}

        fn set_callback(&mut self, callback: Box<dyn CanMsgCtrl>)  {{
            self.callback= Some(RefCell::new(callback));
        }}

        fn get_name(&self) -> &'static str {{
            self.name
        }}

        fn get_status(&self) -> CanBcmOpCode {{
            self.status
        }}

        fn get_stamp(&self) -> u64 {{
            self.stamp
        }}

        fn get_id(&self) -> u32 {{
            self.id
        }}

        fn as_any(&mut self) -> &mut dyn Any {{
            self
        }}

    }} // end {msg_type} impl for CanDbcMessage"#
            )
        )?;

        Ok(())
    }

    fn gen_code_message(&self, code: &DbcCodeGen) -> io::Result<()> {
        // message header
        let name = &self.name;
        let id = self.id.raw();
        let size = self.size;

        code_output!(
            code,
            format!(
                r#"/// {name} Message
/// - ID: {id} (0x{id:x})
/// - Size: {size} bytes"#
            )
        )?;

        if let Transmitter::NodeName(transmitter) = &self.transmitter {
            code_output!(code, format!(r"/// - Transmitter: {transmitter}"))?;
        }

        if let Some(comment) = code.dbcfd.message_comment(self.id) {
            code_output!(code, "///")?;
            for line in comment.trim().lines() {
                code_output!(code, format!(r"/// {line}"))?;
            }
        }

        // per message module/name-space
        let msg_mod = self.get_type_kamel();

        code_output!(
            code,
            format!(
                r#"pub mod {msg_mod} {{ /// Message name space
    use sockcan::prelude::*;
    use bitvec::prelude::*;
    use std::any::Any;
    use std::cell::{{RefCell}};
    use std::rc::Rc;

    use std::fmt;
"#
            )
        )?;

        if code.serde_json {
            code_output!(code, r#"    use serde::{Deserialize, Serialize};"#)?;
        }

        // enumeration with all signal type
        code_output!(code, r#"    pub enum DbcSignal {"#)?;
        for signal in &self.signals {
            let type_id = signal.get_type_kamel();
            code_output!(code, format!(r#"        {type_id},"#))?;
        }
        code_output!(
            code,
            r#"    }
"#
        )?;

        // signals structures and implementation
        for signal in &self.signals {
            signal.gen_code_signal(code, self)?;
        }

        self.gen_can_dbc_impl(code)?;
        self.gen_can_dbc_message(code)?;
        let msg_type = self.get_type_kamel();
        code_output!(
            code,
            format!(
                r#"}} // end {msg_type} message
"#
            )
        )?;
        Ok(())
    }
}

pub trait Text2Str<T> {
    /// Write a line with indentation.
    ///
    /// # Errors
    /// Propagates any I/O error from the underlying writer.
    fn write(&self, indent: &str, text: T) -> io::Result<()>;
}

impl Text2Str<&str> for DbcCodeGen {
    fn write(&self, indent: &str, text: &str) -> io::Result<()> {
        let nl = "\n";
        if let Some(outfd) = &self.outfd {
            let mut outfd = outfd.try_clone()?;
            outfd.write_all(indent.as_bytes())?;
            outfd.write_all(text.as_bytes())?;
            outfd.write_all(nl.as_bytes())?;
        } else {
            let mut outfd = io::stdout();
            outfd.write_all(indent.as_bytes())?;
            outfd.write_all(text.as_bytes())?;
            outfd.write_all(nl.as_bytes())?;
        }

        Ok(())
    }
}

impl Text2Str<String> for DbcCodeGen {
    fn write(&self, indent: &str, text: String) -> io::Result<()> {
        Self::write(self, indent, text.as_str())
    }
}

impl DbcCodeGen {
    fn output<T>(&self, indent: &str, text: T) -> io::Result<()>
    where
        DbcCodeGen: Text2Str<T>,
    {
        Self::write(self, indent, text)
    }
}

pub const DEFAULT_HEADER: &str = r#"// -----------------------------------------------------------------------
//              <- DBC file Rust mapping ->
// -----------------------------------------------------------------------
//  Do not edit this file it will be regenerated automatically by cargo.
//  Check:
//   - build.rs at project root for dynamically mapping
//   - example/demo/dbc-log/??? for static values
//  Reference: iot.bzh/Redpesk canbus-rs code generator
// -----------------------------------------------------------------------

// Tell rustfmt (stable) to skip formatting this whole file
#[rustfmt::skip]

#[allow(
    warnings,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::redundant_field_names,
    clippy::similar_names
)]
"#;

impl DbcParser {
    #[must_use]
    pub fn new(uid: &'static str) -> Self {
        DbcParser {
            uid,
            range_check: true,
            serde_json: true,
            infile: None,
            outfile: None,
            header: None,
            whitelist: None,
            blacklist: None,
        }
    }

    pub fn dbcfile(&mut self, dbcfile: &str) -> &mut Self {
        self.infile = Some(dbcfile.to_owned());
        self
    }

    pub fn outfile(&mut self, outfile: &str) -> &mut Self {
        self.outfile = Some(outfile.to_owned());
        self
    }

    pub fn header(&mut self, header: &'static str) -> &mut Self {
        self.header = Some(header);
        self
    }

    pub fn whitelist(&mut self, canids: Vec<u32>) -> &mut Self {
        self.whitelist = Some(canids);
        self
    }

    pub fn blacklist(&mut self, canids: Vec<u32>) -> &mut Self {
        self.blacklist = Some(canids);
        self
    }

    pub fn range_check(&mut self, flag: bool) -> &mut Self {
        self.range_check = flag;
        self
    }

    pub fn serde_json(&mut self, flag: bool) -> &mut Self {
        self.serde_json = flag;
        self
    }

    fn check_list(canid: MessageId, list: &[u32]) -> bool {
        list.binary_search(&canid.raw()).is_ok()
    }

    /// # Errors
    /// Propagates any I/O error: reading the DBC, parsing, writing output, and time formatting.
    #[allow(clippy::too_many_lines)]
    pub fn generate(&mut self) -> io::Result<()> {
        let Some(infile) = &self.infile else {
            return Err(Error::other("setting dbcpath is mandatory"));
        };

        // open and parse dbc input file
        let buffer = fs::read_to_string(infile.as_str())?;
        let mut dbcfd = match Dbc::try_from(buffer.as_str()) {
            Err(error) => return Err(Error::other(error.to_string())),
            Ok(dbcfd) => dbcfd,
        };

        // sort message by canid
        dbcfd.messages.sort_by(|a, b| a.id.raw().cmp(&b.id.raw()));

        if let Some(mut list) = self.whitelist.clone() {
            if list.is_empty() {
                // empty whitelist means "keep everything"
                dbcfd.messages.retain(|_| true);
            } else {
                list.sort_unstable();
                dbcfd.messages.retain(|msg| DbcParser::check_list(msg.id, &list));
            }
        }

        if let Some(mut list) = self.blacklist.clone() {
            list.sort_unstable();
            dbcfd.messages.retain(|msg| !DbcParser::check_list(msg.id, &list));
        }

        // sort message by canid
        dbcfd.messages.sort_by(|a, b| a.id.raw().cmp(&b.id.raw()));

        let outfd = match &self.outfile {
            Some(outfile) => {
                let outfd = File::create(outfile.as_str())?;
                Some(outfd)
            },
            None => None,
        };

        // open/create output file
        let code =
            DbcCodeGen { dbcfd, outfd, range_check: self.range_check, serde_json: self.serde_json };

        if let Some(header) = self.header {
            code_output!(code, header)?;
        }

        // change Rust default to stick as much as possible on can names
        let gen_time = get_time("%c")?;

        let uid = self.uid;
        code_output!(
            code,
            format!(
                r#"// --------------------------------------------------------------
//       WARNING: Manual modification will be destroyed
// --------------------------------------------------------------
// - code generated from {infile} ({gen_time})
// - update only with [dbc-parser|build.rs::DbcParser]
// - source code: https://github.com/redpesk-common/canforge-rs
// Generated file — DO NOT EDIT.
// Update only with [dbc-parser|build.rs::DbcParser]
// Source: https://github.com/redpesk-common/canforge-rs
//
// Copyright (C) 2023 IoT.bzh Company
// Author: Fulup Ar Foll <fulup@iot.bzh>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// -------------------------------------------------------------
mod {uid} {{
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]"#
            )
        )?;

        if code.serde_json {
            code_output!(code, "extern crate serde;")?;
        }
        code_output!(
            code,
            r#"extern crate bitvec;
use sockcan::prelude::*;
use std::cell::{RefCell,RefMut};
use std::rc::{Rc};
"#
        )?;

        // output messages/signals
        for message in &code.dbcfd.messages {
            message.gen_code_message(&code)?;
        }

        // enumeration with all signal type
        code_output!(code, "enum DbcMessages {")?;
        for message in &code.dbcfd.messages {
            let msg_type = message.get_type_kamel();
            code_output!(code, format!(r#"    {msg_type},"#))?;
        }
        // extract canid from messages vector
        let canids: Vec<u32> = code.dbcfd.messages.iter().map(|msg| msg.id.raw()).collect();

        let msg_count = code.dbcfd.messages.len();

        code_output!(
            code,
            format!(
                r#"}}

pub struct CanMsgPool {{
    uid: &'static str,
    pool: [Rc<RefCell<Box<dyn CanDbcMessage>>>;{msg_count}],
}}

impl CanMsgPool {{
    pub fn new(uid: &'static str) -> Self {{
        CanMsgPool {{
            uid: uid,
            pool: ["#
            )
        )?;

        for idx in 0..code.dbcfd.messages.len() {
            let msg_type = code.dbcfd.messages[idx].get_type_kamel();
            code_output!(code, format!(r#"                {msg_type}::DbcMessage::new(),"#))?;
        }
        code_output!(
            code,
            format!(
                r#"            ]
        }}
    }}
}}

impl CanDbcPool for CanMsgPool {{
    fn get_messages(&self) -> &[Rc<RefCell<Box<dyn CanDbcMessage>>>] {{
        &self.pool
    }}

    fn get_ids(&self) -> &[u32] {{
        &{canids:?}
    }}

    fn get_mut(&self, canid: u32) -> Result<RefMut<'_, Box<dyn CanDbcMessage>>, CanError> {{
        let search= self.pool.binary_search_by(|msg| msg.borrow().get_id().cmp(&canid));
        match search {{
            Ok(idx) => {{
                match self.pool[idx].try_borrow_mut() {{
                    Err(_code) => Err(CanError::new("message-get_mut", "internal msg pool error")),
                    Ok(mut_ref) => Ok(mut_ref),
                }}
            }},
            Err(_) => Err(CanError::new("fail-canid-search", format!("canid:{{}} not found",canid))),
        }}
    }}

    fn update(&self, data: &CanMsgData) -> Result<RefMut<'_, Box<dyn CanDbcMessage>>, CanError> {{
        let mut msg= match self.get_mut(data.canid) {{
            Err(error) => return Err(error),
            Ok(msg_ref) => msg_ref,
        }};
        msg.update(data)?;
        Ok(msg)
    }}
 }}
}} // end dbc generated parser"#
            )
        )?;

        Ok(())
    }
}
