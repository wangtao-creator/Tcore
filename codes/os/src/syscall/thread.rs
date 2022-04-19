use crate::{
    mm::kernel::token,
    task::{add_task,current_task,TaskControlBlock},
    trap::{trap_handler,TaskContext},
};

use alloc::sync::Arc;

pub fn thread_create(entry:usize,args:usize) -> isize {
    let task = current_task.unwrap();




}