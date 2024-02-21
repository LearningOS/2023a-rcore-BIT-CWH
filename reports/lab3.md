# ch5实验报告
## 实验内容
1.在“伤齿龙”操作系统的基础上完成ch3、ch4的实验内容并通过测试用例。  
2.实现sys_spawn系统调用，创建新的子进程，并执行目标程序。  
3.实现sys_set_priority系统调用。  
4.实现stride调度算法用于进程调度。
## 设计思路
### 完成ch3、ch4的实验内容
将原有的为TaskManger实现的方法改为对进程控制块实现的方法，通过对当前任务调用进程控制块对应的方法来完成目标。
### 完成sys_spawn系统调用
1.该系统调用传入一个指向任务名的指针，首先进行参数检查：检查该指针指向的任务名对应任务的文件是否存在，参考指导用书，使用translate_str()方法。  
2.若文件存在，则对当前进程控制块调用spawn()方法，生成一个子进程的控制块，并将其加入任务队列。  
3.spawn()方法参考进程控制块的new()方法，不同点在于其父进程为当前进程，需要将其加入父进程的子进程队列。
### 完成sys_set_priority系统调用
1.在TaskControlBlockInner中新增priority数据结构，表示当前进程的优先级，初始化为16。  
2.在系统调用中，首先应确认传入的优先级数是否合法，即大于等于2。  
3.若参数合法，则为当前进程设置优先级，写入TaskControlBlockInner中。
### 实现stride优先队列调度方法
1.在TaskControlBlockInner中新增stride数据结构，表示当前进程累计的stride值，初始化为0。  
2.在config中配置BIG_STRIDE值为0x4000。  
3.为TaskManager实现stride优先队列选择算法，遍历就绪队列中的进程控制块，取出其中stride值最小的进程，并将其stride加上pass(pass = BIG_STRIDE / priority)，并将其作为调度的选择对象。  
4.在负责进程调度的run_tasks()方法中采用stride选择算法选择下一调度对象。
## 实验代码
### 完成sys_spawn系统调用
1.sys_spawn()系统调用
```rust
/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn",
        current_task().unwrap().pid.0
    );
    //处理path
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        //app存在，则生成进程
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data);
        let new_pid = new_task.pid.0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}
```
2.进程控制块的spawn()方法
```rust
    ///生成执行data的进程，并返回进程的pid
    pub fn spawn (self: &Arc<Self>, elf_data: &[u8]) -> Arc<Self> {
        //获取父进程inner
        let mut parent_inner = self.inner.exclusive_access();
        //生成应用空间memory_set
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set.translate(VirtAddr::from(TRAP_CONTEXT_BASE).into()).unwrap().ppn();
        //分配pid和内核栈
        let pid_handle = pid_alloc();
        let kernel_stack = kstack_alloc();
        let kernel_stack_top = kernel_stack.get_top();
        let task_control_block = Arc::new(TaskControlBlock{
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner{
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    heap_bottom: user_sp,
                    program_brk: user_sp,
                    syscall_times: [0; MAX_SYSCALL_NUM],
                    dispatch_times: 0,
                    first_dispatch_time: 0,
                    priority: 16,
                    stride: 0
                })
            }
        });
        //加入父进程的列表
        parent_inner.children.push(task_control_block.clone());
        //初始化trap_cx
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize
        );
        task_control_block
    }
```
### 完成sys_set_priority系统调用
1.sys_set_priority()方法
```rust
// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if _prio <2 {
        -1
    } else {
        //设置优先级
        let current_task = current_task().unwrap();
        let mut inner = current_task.inner_exclusive_access();
        inner.priority = _prio as usize;
        drop(inner);
        _prio
    }
}
```

### 实现stride优先队列调度方法
1.TaskManager的fetch_by_stride()方法
```rust
    ///使用Stride优先调度算法选择进程
    pub fn fetch_by_stride(&mut self) -> Option<Arc<TaskControlBlock>> {
        if self.ready_queue.len() == 0 {
            return None;
        }
        let mut index = 0;
        let mut smallest_stride = self.ready_queue[index].inner_exclusive_access().stride;
        for (count, task) in self.ready_queue.iter().enumerate() {
            if index == count {
                continue;
            }
            let task_inner = task.inner_exclusive_access();
            if task_inner.stride < smallest_stride {
                smallest_stride = task_inner.stride;
                index = count;
            }
            drop(task_inner);
        }
        //更新被选中进程的信息
        let result = self.ready_queue.remove(index).unwrap();
        let mut inner = result.inner_exclusive_access();
        let prio = inner.priority;
        //stride += pass
        inner.stride += BIG_STRIDE / prio;
        drop(inner);
        Some(result)
    }
```
## 实验结果
在os目录下运行如下命令。
```
make run BASE=2
```
可以通过如下测试用例：  
1.ch3_sleep  
2.ch3_sleep1  
3.ch3_taskinfo  
4.ch4_mmap0  
5.ch4_mmap1  
6.ch4_mmap2  
7.ch4_mmap3  
8.ch4_unmap   
9.ch4_unmap2   
10.ch5_spawn0  
11.ch5_spawn1  
12.ch5_setprio  
13.ch5_stride  
其中，ch5_stride中完成进程的优先级顺序为8、7、6、5、9、10，运行时间与优先级比例分别为1798900、1808457、1802266、1780960、1822577、1820920满足通过的要求：最大值比最小值小于1.5。

## 实验心得总结
在本次实验中，我手动实现了一个新的系统调用用于统计任务信息，对批处理操作系统的构成以及系统调用的运作模式有了深入的认识，受益匪浅，并有了如下编程经验的收获：  
1.对于在新编写的函数中使用exclussive_access()方法获取引用时，如对采用RefCell声明的inner时，其底层调用RefCell的borrow_mut()方法。在使用完后需要手动drop，否则引用会残留在内存中，导致后续调用exclussive_access()会出现already borrowed: BorrowMutError错误。在一个函数中，如果其内部的函数调用也使用了exclussive_access()方法，需要特别注意调用之前的exclussive_access()方法的变量是否drop。  
2.在记录信息或切换函数获取任务控制块的Arc引用时，在使用完后也需手动drop，否则Arc引用的链接计数也仍存在在内存中，导致父进程回收子进程时，子进程的连接计数大于1，无法被正常回收，内核Panic退出。
总之，在Rust编程中需要非常注意每一个变量或引用的生命周期以及其所有权的传递和回收问题，特别是在函数返回和传递参数时。