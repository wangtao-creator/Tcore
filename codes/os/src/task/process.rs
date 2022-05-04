use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::TaskControlBlock;
use super::{add_task, Signal};
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
use spin::Mutex;

pub struct ProcessControlBlock{
    //immutable
    pub pid : PidHandle,
    //mutable
    inner : Mutex<ProcessControlBlockInner>,
}

pub type FdTable =  Vec<Option<FileDescripter>>;

pub struct ProcessControlBlockInner{
    pub is_zombie: bool,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: FdTable,
    pub signals: Signal,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator: RecycleAllocator,
}

impl ProcessControlBlockInner {
    #[allow(unused)]
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
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
                signals:Signal::empty(),
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
        let task_inner  = task.acquire_inner_lock();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().usatck_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.lock().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.acquire_inner_lock();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }
    pub fn exec(self : &Arc<self>,elf_data:&[u8],args:Vec<String>)->{
        assert_eq!(self.inner.lock().thread.count(),0);
        //memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set,user_stack,entry_point)  = MemorySet::from_elf(elf_data);
        let new token  =  memory_set.token();
        //substitute memory_set
        self.acquire_inner_lock().memory_set = memory_set;
         // then we alloc user resource for main thread again
        // since memory_set has been changed
        let task = self.acquire_inner_lock().get_task(0);
        let mut task_inner = task.acquire_inner_lock();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    new_token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(new_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(new_token, p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // initialize trap_cx
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }
     /// Only support processes with a single thread.
     pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent = self.acquire_inner_lock();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table: FdTable = Vec::new();
        for fd in parent.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: Mutex::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: Signal::empty(),
                    tasks: Vec::new(),
                }),
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .acquire_inner_lock()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        // attach task to child process
        let mut child_inner = child.acquire_inner_lock();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.acquire_inner_lock();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}
        









