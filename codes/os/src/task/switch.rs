global_asm!(include_str!("switch.S"));

extern "C" {
    pub fn __switch(
        current_task_cx_ptr: *mut TrapContext,
        next_task_cx_ptr: *const TrapContext
    );
}