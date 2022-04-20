# 初步修改
在other core 加载过程中，task:run_tasks()之前添加了task::other_core_add_initproc()
```
# Tcore/codes/os/src/main.rs

task::other_core_add_initproc();
task::run_tasks();


# Tcore/codes/os/src/task/mod.rs

pub fn other_core_add_initproc() {
    add_task(INITPROC.clone());
}

```

但是在make run过程中，有几率出现panic。或者卡住，无法运行到user_shell
```
[kernel] Panicked at src/trap/mod.rs:247 a trap Exception(StorePageFault) from kernel! Stvec:80221dbc, Stval:0
[kernel] Panicked at src/trap/mod.rs:247 a trap Exception(LoadPageFault) from kernel! Stvec:80221dbc, Stval:0

or

[kernel] Panicked at src/task/processor.rs:183 
```
## 流程梳理
两个core是从一个TASKMANAGER中取TaskControlBlock，分别通过不同的Processor.run()运行。
这也印证，上面的初步修改应该是没有必要的。