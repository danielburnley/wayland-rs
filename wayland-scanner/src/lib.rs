//! Wayland scanner crate
//!
//!
//! This crate is a rust equivalent of the wayland-scanner tool from the
//! official wayland C library.
//!
//! You can use in your build script to generate the rust code for any
//! wayland protocol file, to use alongside the `wayland_client` and
//! `wayland_server` crate to build your applications.
//!
//! ## How to use this crate
//!
//! This crate is to be used in a build script. It provides you two
//! functions, `generate_code` and `generate_interfaces`. They'll
//! allow you to generate the code to use with the `wayland_client` or
//! `wayland_server` crates for any XML wayland protocol file (NB: you don't
//! need to do it for the core protocol, which is already included in both crates).
//!
//! First, have the XML files you want to use in your project, somewhere the build script
//! will be able to read them.
//!
//! Then, you'll need to invoke both `generate_interfaces` *and* `generate_code` for
//! each of these files.
//!
//! A sample build script:
//!
//! ```no_run
//! extern crate wayland_scanner;
//!
//! use std::env::var;
//! use std::path::Path;
//!
//! use wayland_scanner::{Side, generate_code, generate_interfaces};
//!
//! fn main() {
//!     // Location of the xml file, relative to the `Cargo.toml`
//!     let protocol_file = "./my_protocol.xml";
//!
//!     // Target directory for the generate files
//!     let out_dir_str = var("OUT_DIR").unwrap();
//!     let out_dir = Path::new(&out_dir_str);
//!
//!     generate_code(
//!         protocol_file,
//!         out_dir.join("my_protocol_api.rs"),
//!         Side::Client, // Replace by `Side::Server` for server-side code
//!     );
//!
//!     // interfaces are the same for client and server
//!     generate_interfaces(
//!         protocol_file,
//!         out_dir.join("my_protocol_interfaces.rs")
//!     );
//! }
//! ```
//!
//! The above example will output two `.rs` files in the `OUT_DIR` defined by
//! cargo. Then, you'll need to include these two generated files (using the
//! macro of the same name) to make this code available in your crate.
//!
//! ```ignore
//! // The generated code will import stuff from wayland_sys
//! extern crate wayland_sys;
//! extern crate wayland_client;
//!
//! // Re-export only the actual code, and then only use this re-export
//! // The `generated` module below is just some boilerplate to properly isolate stuff
//! // and avoid exposing internal details.
//! //
//! // You can use all the types from my_procol as if they went from `wayland_client::protocol`.
//! pub use generated::client as my_protocol;
//!
//! mod generated {
//!     // The generated code tends to trigger a lot of warnings
//!     // so we isole it into a very permissive module
//!     #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
//!     #![allow(non_upper_case_globals,non_snake_case,unused_imports)]
//!
//!     pub mod interfaces {
//!         // include the interfaces, they just need to be accessible to the generated code
//!         // so this module is `pub` so that it can be imported by the other one.
//!         include!(concat!(env!("OUT_DIR"), "/my_protocol_interfaces.rs"));
//!     }
//!
//!     pub mod client {
//!         // These import are mandatory, and need to by `pub`, because
//!         // submodules in the generated code will try to import them.
//!         // Hopefully someday pub(restricted) and friends will
//!         // allow for a cleaner way to do that
//!         #[doc(hidden)] pub use wayland_client::{Proxy, Handler, EventQueueHandle, RequestResult};
//!         // Replace the above line with this for server-side:
//!         // #[doc(hidden)] pub use wayland_server::{Resource, Handler, EventLoopHandle, EventResult};
//!         #[doc(hidden)] pub use super::interfaces;
//!         // If you protocol interacts with objects from other protocols, you'll need to import
//!         // their modules, like so:
//!         #[doc(hidden)] pub use wayland_client::protocol::{wl_surface, wl_region};
//!         include!(concat!(env!("OUT_DIR"), "/my_protocol_code.rs"));
//!     }
//! }
//! ```

#![warn(missing_docs)]

extern crate xml;

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

mod util;
mod parse;
mod protocol;
mod side;
mod interface_gen;
mod code_gen;

pub use side::Side;

fn load_xml<P: AsRef<Path>>(prot: P) -> protocol::Protocol {
    let pfile = File::open(prot).unwrap();
    parse::parse_stream(pfile)
}

/// Generate the interfaces for a protocol
///
/// See this crate toplevel documentation for details.
///
/// Args:
///
/// - `protocol`: a path to the XML file describing the protocol, absolute or relative to
///   the build script using this function.
/// - `target`: the path of the file to store this interfaces in.
pub fn generate_interfaces<P1: AsRef<Path>, P2: AsRef<Path>>(protocol: P1, target: P2) {
    let protocol = load_xml(protocol);
    let mut out = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(target)
        .unwrap();
    interface_gen::generate_interfaces(protocol, &mut out);
}

/// Generate the code for a protocol
///
/// See this crate toplevel documentation for details.
///
/// Args:
///
/// - `protocol`: a path to the XML file describing the protocol, absolute or relative to
///   the build script using this function.
/// - `target`: the path of the file to store the code in.
/// - `side`: the side (client or server) to generate code for.
pub fn generate_code<P1: AsRef<Path>, P2: AsRef<Path>>(prot: P1, target: P2, side: Side) {
    let protocol = load_xml(prot);
    let mut out = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(target)
        .unwrap();
    code_gen::write_protocol(protocol, &mut out, side).unwrap()
}

/// Generate the interfaces for a protocol from/to IO streams
///
/// Like `generate_interfaces`, but takes IO Streams directly rather than filenames
///
/// Args:
///
/// - `protocol`: an object `Read`-able containing the XML protocol file
/// - `target`: a `Write`-able object to which the generated code will be outputed to
pub fn generate_interfaces_streams<P1: Read, P2: Write>(protocol: P1, target: &mut P2) {
    let protocol = parse::parse_stream(protocol);
    interface_gen::generate_interfaces(protocol, target);
}

/// Generate the code for a protocol from/to IO streams
///
/// Like `generate_code`, but takes IO Streams directly rather than filenames
///
/// Args:
///
/// - `protocol`: an object `Read`-able containing the XML protocol file
/// - `target`: a `Write`-able object to which the generated code will be outputed to
/// - `side`: the side (client or server) to generate code for.
pub fn generate_code_streams<P1: Read, P2: Write>(protocol: P1, target: &mut P2, side: Side) {
    let protocol = parse::parse_stream(protocol);
    code_gen::write_protocol(protocol, target, side).unwrap()
}
