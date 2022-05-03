mod context;
mod id;
mod manager;
mod process;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::fs::{open_file, OpenFlags};
use alloc::sync::Arc;
use lazy_static::*;
use manager::fetch_task;
use process::ProcessControlBlock;
use switch::__switch;