//! Argument utilities for QEMU plugins

use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    ffi::{c_char, CStr},
};

lazy_static! {
    static ref TRUE_STRINGS: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("true".to_string());
        set.insert("1".to_string());
        set.insert("yes".to_string());
        set.insert("on".to_string());
        set
    };
    static ref FALSE_STRINGS: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("false".to_string());
        set.insert("0".to_string());
        set.insert("no".to_string());
        set.insert("off".to_string());
        set
    };
}

#[derive(Debug)]
pub enum QEMUArg {
    Bool(bool),
    Int(i64),
    Str(String),
}

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

#[derive(Debug)]
pub struct Args {
    raw: Vec<String>,
    args: HashMap<String, QEMUArg>,
}

impl Args {
    pub fn new(argc: i32, argv: *const *const c_char) -> Self {
        let mut raw = Vec::new();
        for i in 0..argc {
            let arg = unsafe { CStr::from_ptr(*argv.offset(i as isize)) };
            raw.push(arg.to_string_lossy().to_string());
        }

        let mut args = HashMap::new();
        for arg in raw.iter().skip(1) {
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
