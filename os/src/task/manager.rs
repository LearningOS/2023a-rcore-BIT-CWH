//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }

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
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
// pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
//     //trace!("kernel: TaskManager::fetch_task");
//     TASK_MANAGER.exclusive_access().fetch()
// }

///Stride优先级调度
pub fn fetch_task_by_stride() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch_by_stride()
}
