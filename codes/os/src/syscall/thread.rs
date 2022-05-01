use crate::{
    mm::kernel::token,
    task::{add_task, current_task, TaskControlBlock},
    trap::{trap_handler, TaskContext},
};

use alloc::sync::Arc;

pub fn thread_create(entry: usize, args: usize) -> isize{
    //find the executing thread and the process  it belongs to
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    //crate a new thread
    let new_task = Arc::new(TaskControlBlock::new(
        Arc::clone(&process),
        task.acquire_inner_lock()
            .res
            .as_ref()
            .unwrap()
            .ustack_base,
        true,
    ));
    // add new thread to scheduler
    add_task(Arc::clone(&new_task));
    let new_task_inner = new_task.acquire_inner_lock();
    let new_task_res = new_task_inner.res.as_ref().unwrap();
    let new_task_tid = new_task_res.tid;
    let mut process_inner = process.inner();
    //add new thread to current process
    let tasks = &mut process_inner.tasks;
    while tasks.len() < new_task_tid + 1 {
        tasks.push(None);
    }
    tasks[new_task_tid] = Some(Arc::clone(&new_tas));
    let new_task_trap_cx = new_task_inner_get_trap_cx();
    *new_task_trap_cx = TrapContext::app_init_context(
        entry,
        new_task.res_ustack_top(),
        kernel_token(),
        new_task.kstack_get_top(),
        trap_handler as usize,
    );
    (*new_task_trap_cx).x[10] = args;
    new_task_tid as isize
}

pub fn sys_gettid() -> isize {
    current_task()
        .unwrap()
        .inner_lock()
        .res
        .as_ref()
        .unwrap()
        .tid as isize
}

/// thread does not exist, return -1
/// thread has not exited yet, return -2
/// otherwise, return thread's exit code
pub fn sys_waittid(tid: usize) -> i32 {
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    let task_inner = task.acquire_inner_lock();
    let mut process_inner = process.inner.lock();
    // a thread cannot wait for itself
    if task_inner.res.as_ref().unwrap().tid == tid {
        return -1;
    }
    let mut exit_code: Option<i32> = None;
    let waited_task = process_inner.wait[tid].as_ref();
    if let Some(waited_task) = waited_task {
        if let Some(waited_exit_code) = waited_task.inner().exit_code {
            exit_code = Some(waited_exit_code);
        }
    }else{
        //waited thread  does not exited
        return -1;
    }
    if let Some(exit_code) =exit_code {
        //dealloc the exited thread
        process_inner.task[tid] = None;
        exit_code
    }else{
        // waited thread has not exited
        -2
}
