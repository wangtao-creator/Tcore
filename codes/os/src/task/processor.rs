// #![feature(llvm_asm)]
// #[macro_use]
use super::{ProcessControlBlock, TaskContext,TaskControlBlock, RUsage};
use alloc::sync::Arc;
use core::{borrow::Borrow, cell::RefCell};
use lazy_static::*;
use super::{fetch_task, TaskStatus, Signals, SIG_DFL};
use super::__switch;
use crate::timer::get_time_us;
use crate::trap::TrapContext;
use crate::task::manager::add_task;
use crate::gdb_print;
use crate::monitor::*;

pub fn get_core_id() -> usize {
    let tp :usize = get_core_id();
    unsafe {
        llvm_asm!("mv $0,tp" : "=r"(tp));
    }
    //tp
    0
}


pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
    user_clock: usize,  /* Timer usec when last enter into the user program */
    kernel_clock: usize, /* Timer usec when user program traps into the kernel*/
}

impl Processor {
    pub fn new() -> Self {
        Self {
                current: None,
                idle_task_cx: TaskContext::zero_init(),
                user_clock: 0,  
                kernel_clock: 0,
        }
    }

    // when trap return to user program, use this func to update user clock
    pub fn update_user_clock(& self){
        self.user_clock = get_time_us();
    }
    
    // when trap into kernel, use this func to update kernel clock
    pub fn update_kernel_clock(& self){
        self.kernel_clock = get_time_us();
    }

    pub fn get_user_clock(& self) -> usize{
        return self.user_clock;
    }

    pub fn get_kernel_clock(& self) -> usize{
        return self.kernel_clock;
    }

    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    pub fn run(&self) {
        loop{
            
            if let Some(task) = fetch_task() {
                let idle_task_cx_ptr = self.get_idle_task_cx_ptr();
                // access coming task TCB exclusively
                let mut task_inner = task.acquire_inner_lock();
                let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
                task_inner.task_status = TaskStatus::Running;
                drop(task_inner);
                // release coming task TCB manually
                processor.current = Some(task);
                // release processor manually
                drop(processor);
                unsafe {
                    __switch(idle_task_cx_ptr, next_task_cx_ptr);
                }
            } else {
                println!("no tasks available in run_tasks");
            }
        }
    }
     pub fn take_current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(|task| Arc::clone(task))
    }
}

lazy_static! {
    pub static ref PROCESSOR_LIST: [Processor; 2] = [Processor::new(),Processor::new()];
}

pub fn run_tasks() {
    let core_id: usize = get_core_id();
    PROCESSOR_LIST[core_id].run();
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    let core_id: usize = get_core_id();
    PROCESSOR_LIST[core_id].take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    let core_id: usize = get_core_id();
    PROCESSOR_LIST[core_id].current()
}
pub fn current_process() -> Arc<ProcessControlBlock> {
    current_task().unwrap().process.upgrade().unwrap()
}



pub fn current_user_token() -> usize {
    // let core_id: usize = get_core_id();
    let task = current_task().unwrap();
    let token = task.get_user_token();
    token
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().acquire_inner_lock().get_trap_cx()
}

// when trap return to user program, use this func to update user clock
pub fn update_user_clock(){
    let core_id: usize = get_core_id();
    PROCESSOR_LIST[core_id].update_user_clock();
}

// when trap into kernel, use this func to update kernel clock
pub fn update_kernel_clock(){
    let core_id: usize = get_core_id();
    PROCESSOR_LIST[core_id].update_kernel_clock();
}

// when trap into kernel, use this func to get time spent in user (it is duration not accurate time)
pub fn get_user_runtime_usec() -> usize{
    let core_id: usize = get_core_id();
    return get_time_us() - PROCESSOR_LIST[core_id].get_user_clock();
}

// when trap return to user program, use this func to get time spent in kernel (it is duration not accurate time)
pub fn get_kernel_runtime_usec() -> usize{
    let core_id: usize = get_core_id();
    return get_time_us() - PROCESSOR_LIST[core_id].get_kernel_clock();
}


pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let core_id: usize = get_core_id();
    let idle_task_cx_ptr= PROCESSOR_LIST[core_id].get_idle_task_cx_ptr();
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}