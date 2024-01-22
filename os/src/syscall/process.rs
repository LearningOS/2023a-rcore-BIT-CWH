//! Process management syscalls
use core::mem::size_of;

use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, current_user_token, TASK_MANAGER,
    }, mm::{translated_byte_buffer, VirtAddr, VirtPageNum}, timer::get_time_us,
};

///时间结构
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    ///秒
    pub sec: usize,
    ///微秒
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    //获取物理地址引用
    let contents = translated_byte_buffer(current_user_token(), _ts as *const u8, size_of::<TimeVal>());
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000
    };
    let  mut time_val_ptr =  &time_val as *const _ as *const u8;
    //按字节写入
    for content in contents {
        unsafe {
            time_val_ptr.copy_to(content.as_mut_ptr(), content.len());
            time_val_ptr = time_val_ptr.add(content.len());
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    //获取物理地址引用
    let contents = translated_byte_buffer(current_user_token(), _ti as *const u8, size_of::<TaskInfo>());
    let info = TASK_MANAGER.show_info();
    let  mut info_ptr =  &info as *const _ as *const u8;
    //按字节写入
    for content in contents {
        unsafe {
            info_ptr.copy_to(content.as_mut_ptr(), content.len());
            info_ptr = info_ptr.add(content.len());
        }
    }
    0
}

///YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len - 1);
    //检查start是否与页面对齐
    if !start_va.aligned() {
        return -1;
    }
    let start_vpn = VirtPageNum::from(start_va);
    let end_vpn = end_va.floor();
    //检查port是否合法，检查[start-start+len]是否已有映射
    if (_port & !0x7 != 0) || (_port & 0x7 == 0) || TASK_MANAGER.check_map_overlap(start_vpn, end_vpn){
        return -1;
    }
    //内存分配
    TASK_MANAGER.alloc_mem_map(start_va, end_va, _port);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len - 1);
    //检查start是否与页面对齐
    if !start_va.aligned() {
        return -1;
    }
    let start_vpn = VirtPageNum::from(start_va);
    let end_vpn = end_va.floor();
    //检查port是否合法，检查[start-start+len]是否已有映射
    TASK_MANAGER.check_map_then_unmap(start_vpn, end_vpn)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
