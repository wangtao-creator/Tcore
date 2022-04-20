use crate::{
    mm::kernel::token,
    task::{add_task, current_task, TaskControlBlock},
    trap::{trap_handler, TaskContext},
};

use alloc::sync::Arc;

pub fn thread_create(entry: usize, args: usize) -> isize {
    let task = current_task.unwrap();
    let process = task.process.upgrade().unwrap();
    //crate a new thread
    let new_task = Arc::new(TaskControlBlock::new(
        Arc::clone(&process),
        task.inner_exclusive_lock()
            .res
            .as_ref()
            .unwrap()
            .ustack_base,
        true,
    ));
    // add new thread to scheduler
    add_task(Arc::clone(&new_task));
    let new_task_inner = new_task.inner_exclusive_access();
    let new_task_res = new_task_inner.res.as_ref().unwrap();
    let new_task_tid  = new_task_res.tid;
    let mut process_inner = process.inner_exclusive_access();
    //add new thread to current process
    let tasks = &mut process_inner.tasks;
    while tasks.len() <new_task_tid + 1{
        tasks.push(None);
    }
    tasks[new_task_tid] = Some(Arc::clone(&new_tas));
    let new_task_trap_cx = new_task_inner_get_trap_cx();
    *new_task_trap_cx =Trap
}
