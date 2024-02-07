# ch3实验报告
## 实验内容
为系统引入新的系统调用sys_task_info，查询当前正在执行的任务信息，任务信息包括任务控制块相关信息（任务状态）、任务使用的系统调用以及调用次数、系统调用时刻距离人物第一次被调度时刻的时常（单位ms）。
## 设计思路
### 数据结构
1.由于系统调用号一定小于500，因此在任务控制块中加入一个大小为MAX_SYSCALL_NUM的数组用于统计每个系统调用被当前任务调用的次数。  
2.为了实现对当前调用系统调用的时间距离初次调度时间的统计，在任务控制块中加入一个内容usize的变量用于记录当前任务被调度次数以及一个内容为usize的变量用于记录初次调度时间。
### 方法与接口
1.为管理任务的全局变量TASK_MANAGER实现一个用于记录系统调用的方法，并在系统调用分发处理的过程中调用此方法，对本次调用进行记录，写入当前任务的控制块。  
2.为TASK_MANAGER实现一个用于记录调度次数的方法，并在系统进行任务调度的时刻调用，当当前任务是初次被调度时，将时间记录进任务控制块，并修改任务控制块中的被调度次数。  
3.为TASK_MANAGER实现一个生成TaskInfo的方法，将当前任务的信息打包封装成TaskInfo并返回，以供实验要求待实现的sys_task_info使用。
## 实验代码
### 1.任务控制块更新
```rust
///os/src/task/task.rs
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    ///各系统调用被调用次数
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    ///调度次数
    pub dispatch_times: usize,
    /// 初次调度时间
    pub first_dispatch_time: usize
}
```
### 2.为TaskManager实现方法
1.update_syscall_record()方法用于记录系统调用次数。  
2.record_time()方法用于记录任务调度次数以及初次调度时间。  
3.show_info()方法用于对外返回当前任务信息。  
4.run_first_task()与run_next_task()新增对record_time()方法的调用以实现对调度信息的记录
```rust
///os/src/task/task.rs
impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        self.record_time();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
           self.record_time();
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
    ///Update system call record 
    fn update_syscall_record(&self, id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[id] += 1;
        drop(inner);
    }
    ///返回TaskInfo
    pub fn show_info(&self) -> TaskInfo {
        let current = self.inner.exclusive_access().current_task;
        let  inner = self.inner.exclusive_access();
        TaskInfo {
            status: TaskStatus::Running,
            syscall_times: inner.tasks[current].syscall_times.clone(),
            time: get_time_ms() - inner.tasks[current].first_dispatch_time
        }
    }
    ///记录调度时间与次数
    pub fn record_time(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        match inner.tasks[current].dispatch_times {
            0 => {
                inner.tasks[current].dispatch_times += 1;
                inner.tasks[current].first_dispatch_time = get_time_ms()
            }
            _ => {

            }
        }
        drop(inner)
    }
    
}
```
## 实验结果
在os目录下运行如下命令，可以通过本地测试。
```
make run BASE=0
```
## 实验总结
在本次实验中，我手动实现了一个新的系统调用用于统计任务信息，对批处理操作系统的构成以及系统调用的运作模式有了深入的认识，受益匪浅。