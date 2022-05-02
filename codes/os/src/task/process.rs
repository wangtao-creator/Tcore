use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::TaskControlBlock;
use super::{add_task, SignalFlags};
use super::{pid_alloc, PidHandle};
use super::{RLimit, TaskContext};
use crate::mm::{translated_refmut, MemorySet, KERNEL_SPACE};
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;
use create::{syscall::FD_LIMIT,RLIMIT_NOFILE};
use crate::fs::{ FileDescripter , Stdin, Stdout};

pub struct ProcessControlBlock{
    //immutable
    pub pid : PidHandle,
    //mutable
    inner : Mutex<ProcessControlBlockInner>,
}

pub type FdTable =  Vec<Option<FileDescripter>>;
pub struct ProcessControlBlockInner{
    pub is_zombie : bool,
    pub memory_set : MemorySet,
    pub parent : Option<ProcessControlBlock>,
    pub children : Vec<Arc<TaskControlBlock>,
    pub exit_code : i32,
    pub current_path: String,
    pub fd_table : FdTable,
    pub tasks : Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator :  RecycleAllocator,
    pub signals : SignalFlags,
}

impl  ProcessControlBlockInner{
    #[allow_unused]
    pub fn get_user_token(&self) ->usize{
        self.memory_set.token()
    }
    pub fn alloc_fd(&mut self) ->usize{
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()){
            fd
        }else{
            self.fd_table.push(None)
            self.fd_table.len() - 1
        }
    }
    pub fn alloc_tid(&mut self) -> usize{
        self.task_res_allocator.alloc()
    }
    pub fn dealloc_tid(&mut self,tid: usize) -> usize{
        self.task_res_allocator.dealloc(tid)
    }
    pub fn thread_count(&self) -> usize{
        self.tasks.len()
    }
    pub fn get_task(&self ,tid: usize) ->Arc<TaskControlBlock>{
        self.tasks.[tid].as_ref().unwrap().clone()
    }
}

impl ProcessControlBlock{
    pub fn acquire_inner_lock(&self) ->RefMut<'_,ProcessControlBlockInner>{
        self.inner.lock()
    }
    pub fn new (elf_data:&[u8])->Arc<Self>{
        //memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set,ustack_base,entry_point) =MemorySet::from_elf(elf_data);
        //alloc  a pid  
        let pid_handle = pid_alloc();
        let process  = Arc::new(Self){
            pid : pid_handle,
            inner :Mutex::new(ProcessControlBlockInner{
                is_zombie : false,
                memory_set, 
                parent:None,
                children :Vec::new(),
                exit_code : 0,
                fd_table: vec![
                    // 0 -> stdin
                    Some( FileDescripter::new(
                        false,
                        FileClass::Abstr(Arc::new(Stdin)) 
                    )),
                    // 1 -> stdout
                    Some( FileDescripter::new(
                        false,
                        FileClass::Abstr(Arc::new(Stdout)) 
                    )),
                    // 2 -> stderr
                    Some( FileDescripter::new(
                        false,
                        FileClass::Abstr(Arc::new(Stdout)) 
                    )),
                ],
                current_path: String::from("/"), // 只有initproc在此建立，其他进程均为fork出
                //should we use it
                //resource_list: [RLimit::new();17],
                signals:SignalFlags::empty(),
                tasks:Vec::new(),
                task_res_allocator :RecycleAllocator::new(),
            }),
        };
        //create a main thread ,we should alloc ustack and trap_cx header
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));
        //prepare trap_cx of main thread
        let task_inner  = task.inner.lock();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().usatck_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }
}