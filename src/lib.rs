// This is free and unencumbered software released into the public domain.

//! This crate implements a `jq` wrapper.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(feature = "jaq")]
mod jaq;
#[cfg(feature = "jaq")]
pub use jaq::*;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
