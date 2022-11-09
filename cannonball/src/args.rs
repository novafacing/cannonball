//! Argument utilities for QEMU plugins

use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    ffi::{c_char, CStr},
};

lazy_static! {
    /// Strings representing a true value that will be parsed into a `true` value
    static ref TRUE_STRINGS: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("true".to_string());
        set
    };
    /// Strings representing a false value that will be parsed into a `false` value
    static ref FALSE_STRINGS: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("false".to_string());
        set
    };
}

#[derive(Debug, Clone)]
/// A wrapper around a QEMU plugin argument
pub enum QEMUArg {
    Bool(bool),
    Int(i64),
    Str(String),
}

/// A value parsed form a QEMU argument
impl QEMUArg {
    pub fn new(arg: &str) -> Self {
        if TRUE_STRINGS.contains(arg) {
            QEMUArg::Bool(true)
        } else if FALSE_STRINGS.contains(arg) {
            QEMUArg::Bool(false)
        } else if let Ok(int) = arg.parse::<i64>() {
            QEMUArg::Int(int)
        } else {
            QEMUArg::Str(arg.to_string())
        }
    }
}

#[derive(Debug, Clone)]
/// Thin wrapper around the arguments passed to the QEMu plugin
pub struct Args {
    /// Raw arguments as passed in to the plugin
    pub raw: Vec<String>,
    /// Parsed arguments as a key-value mapping
    pub args: HashMap<String, QEMUArg>,
}

impl Args {
    /// Instantiate a new `Args` from the raw arguments passed to the plugin
    ///
    /// # Arguments
    ///
    /// * `argc` - The number of arguments
    /// * `argv` - Pointer to the arguments of the form `key=value`
    pub fn new(argc: i32, argv: *const *const c_char) -> Self {
        let mut raw = Vec::new();
        for i in 0..argc {
            let arg = unsafe { CStr::from_ptr(*argv.offset(i as isize)) };
            raw.push(arg.to_string_lossy().to_string());
        }

        let mut args = HashMap::new();
        for arg in raw.iter() {
            let mut split = arg.splitn(2, '=');
            if let Some(key) = split.next() {
                if let Some(value) = split.next() {
                    args.insert(key.to_string(), QEMUArg::new(value));
                }
            }
        }

        Self { raw, args }
    }
}
