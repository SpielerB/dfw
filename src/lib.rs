// Copyright 2017 Pit Kleyersburg <pitkley@googlemail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified or distributed
// except according to those terms.

//! # DFWRS - Docker Firewall Framework in Rust
//!
//! `dfwrs` is conceptually based on the [Docker Firewall Framework, `dfwfw`][dfwfw-github]. Its
//! goal is to make firewall administration with Docker simpler, but also more extensive by trying
//! to replace the Docker built-in firewall handling by direct interaction with iptables.
//!
//! This is accomplished by a flexible configuration which defines how the firewall should be built
//! up. While DFWRS is running, Docker container events will be monitored and the rules rebuilt
//! when necessary.
//!
//! See [DFWFW's README][dfwfw-readme] for more insight. Most of what you will read there will be
//! applicable to DFWRS.
//!
//! ## Configuration
//!
//! The general configuration happens across six categories:
//!
//! * `defaults`
//!
//!     This category defines global, default values to be used by DFWRS and the other categories.
//!
//! * `container_to_container`
//!
//!     This controls the communication between containers and across [Docker
//!     networks][docker-networks].
//!
//! * `container_to_wider_world`
//!
//!     This controls if and how containers may access the wider world, i.e. what they can
//!     communicate across the `OUTPUT` chain on the host.
//!
//! * `container_to_host`
//!
//!     To restrict or allow access to the host, this section is used.
//!
//! * `wider_world_to_container`
//!
//!     This controls how the wider world, i.e. whatever comes in through the `INPUT` chain on the
//!     host, can communicate with a container or a Docker network.
//!
//! * `container_dnat`
//!
//!     This category allows you to define specific rules for destination network address
//!     translation, even or especially across Docker networks.
//!
//! One category which DFWFW covers that is not (yet) implemented in DFWRS is
//! `container_internals`, that is configuring iptables rules within containers.
//!
//! See the [examples][examples] *(TODO)*, and the [configuration types][types.rs] for a detailed
//! description of every configuration section.
//!
//! ## Supported Docker versions
//!
//! At least Docker 1.13.0 is required.
//!
//! DFWRS has been successfully tested under the following Docker versions:
//!
//! * `1.13.0`
//!
//! * `1.13.1`
//!
//! * `17.03.0-ce`
//!
//! * `17.03.1-ce`
//!
//! * `17.04.0-ce`
//!
//! * `17.05.0-ce-rc2`
//!
//! ## Installation
//!
//! While you can use Cargo to install `dfwrs` as a binary, using the Docker image is the preferred
//! way to go, especially if you don't want to install Rust and Cargo on your host:
//!
//! ```console
//! $ docker pull pitkley/dfwrs:0.2
//! $ docker run -d \
//!       --name=dfwrs \
//!       -v /var/run/docker.sock:/var/run/docker.sock:ro \
//!       -v /path/to/your/config:/config \
//!       --net host --cap-add=NET_ADMIN \
//!       pitkley/dfwrs --config-path /config
//! ```
//!
//! This will download a lightweight image, coming in at under 6 MB, and subsequently run it using
//! your configuration.
//!
//! ## Motivation for this reimplementation
//!
//! I have reimplemented DFWFW in Rust for two reasons:
//!
//! 1. DFWFW had lost compatibility with the Docker API starting with release 17.04.0-ce, although
//!    this [has been fixed][dfwfw-issue-13] in the meantime.
//!
//! 2. The main reason for this reimplementation was that I found a real-life project to tackle with
//!    Rust. This project allowed me to delve into quite a few different aspects and facets of Rust
//!    and especially its eco-system, amongst others:
//!
//!   * [`clap`][crates-clap], for parsing of command line arguments
//!   * [`chan`][crates-chan], for easy messaging and coordination between threads
//!   * [`error-chain`][crates-error-chain], for simplified application wide error handling
//!   * [Serde][crates-serde], for deserialization of the TOML configuration
//!   * [`slog`][crates-slog], for structured logging
//!
//!     Disregarding the obvious hair-pulling moments regarding ownership, borrowing and lifetimes,
//!     my experience with Rust and its brillant eco-system has been an absolute pleasure.
//!
//! ## License
//!
//! DFWRS is licensed under either of
//!
//! * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
//!   http://www.apache.org/licenses/LICENSE-2.0)
//! * MIT license ([LICENSE-MIT](LICENSE-MIT) or
//!   http://opensource.org/licenses/MIT)
//!
//! at your option.
//!
//! ### Contribution
//!
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in DFWRS by you, as defined in the Apache-2.0 license, shall be
//! dual licensed as above, without any additional terms or conditions.
//!
//!
//! [crates-clap]: https://crates.io/crates/clap
//! [crates-chan]: https://crates.io/crates/chan
//! [crates-error-chain]: https://crates.io/crates/error-chain
//! [crates-serde]: https://crates.io/crates/serde
//! [crates-slog]: https://crates.io/crates/slog
//!
//! [dfwfw-github]: https://github.com/irsl/dfwfw
//! [dfwfw-issue-13]: https://github.com/irsl/dfwfw/issues/13
//! [dfwfw-readme]: https://github.com/irsl/dfwfw/blob/master/README.md
//!
//! [docker-networks]: https://docs.docker.com/engine/userguide/networking/
//!
//! [moby-issue-32686]: https://github.com/moby/moby/issues/32686
//!
//! [types.rs]: types/index.html

// Increase the compiler's recursion limit for the `error_chain` crate.
#![recursion_limit = "1024"]
#![deny(missing_docs)]

// Import external libraries
#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate error_chain;
extern crate glob;
extern crate iptables as ipt;
extern crate libc;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate shiplift;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;
extern crate time;
extern crate toml;
extern crate url;

// declare modules
pub mod errors;
pub mod iptables;
pub mod process;
pub mod types;
pub mod util;

// re-export process types

pub use process::*;