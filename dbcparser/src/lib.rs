// dbcparser/src/lib.rs

#![doc(
    html_logo_url = "https://iot.bzh/images/defaults/company/512-479-max-transp.png",
    html_favicon_url = "https://iot.bzh/images/defaults/favicon.ico"
)]

// gencode + exports
#[path = "gencode.rs"]
pub mod gencode;

pub use crate::gencode::*;

pub mod prelude {
    pub use crate::gencode::*;
}

