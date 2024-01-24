# ch4实验报告
## 实验内容
1.由于引入了虚存系统，因此程序内变量无法通过地址直接访问，需要为系统重写系统调用sys_get_time与sys_task_info。  
2.实现用于在内存中映射文件的系统调用sys_mmap与sys_munmap，本次实验简化功能，仅用于申请内存。
## 设计思路
### 重写sys_get_time与sys_task_info
1.实验设计建立在ch3新增的数据结构与方法的基础上。  
2.参考教程中sys_write方法的重写，在获取当前时间与任务进程块中信息的基础上，通过调用系统提供的方法translate_byte_buffer()获得系统所分配的物理页面的指针数组，并将信息写入指针所指的区域中。
### 完成sys_mmap与sys_munmap
#### sys_mmap
1.首先进行参数检查：检查传入首地址是否与页面对齐；检查传入prot参数是否高位全为0；检查传入prot参数是否低三位不全为0。  
2.为TASK_MANAGER实现一个用于检测当前任务的要求映射的虚存空间是否已经存在对某个物理区域的映射，通过MemorySet的translate()方法对范围内的虚拟页号进行映射，如果映射成功则说明该范围内有的虚存已被映射至物理内存，出错。  
3.为TASK_MANAGER实现一个为当前任务要求的虚存空间分配物理内存并建立映射的方法，需要将prot参数转化为pte表中的Mapermission格式，并调用MemorySet提供的insert_framed_area()方法。
#### sys_munmap
1.首先进行参数检查：检查传入地址是否与页面对齐。
2.为MemorySet实现通过首尾虚页号检查并解除映射的方法，遍历MemorySet中的内存areas，若某个area的首尾虚页号与本次unmap匹配，则释放当前area，并将其从areas数组中删除。
## 实验代码
### 1.重写sys_get_time与sys_task_info
```rust
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
```
### 实现sys_mmap
1.sys_mmap()方法
```rust
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
```
2.辅助函数check_map_overlap()用于检查对应虚存空间是否已有映射
```rust
    ///检查虚存段映射是否重合，页号查询按闭区间查询
    pub fn check_map_overlap(&self, start_vpn: VirtPageNum, end_vpn: VirtPageNum) -> bool {
        let mut flag = false;
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        for vpn in start_vpn.0 ..= end_vpn.0 {
            if let Some(pte) = inner.tasks[current].memory_set.translate(vpn.into()) {
                if pte.is_valid() {
                    flag = true;
                    break;
                }
            }
        } 
        flag
    } 
```
3.辅助函数alloc_mem_map()用于生成MapPermission字段并申请内存
```rust
    ///分配映射
    pub fn alloc_mem_map(&self, start_va: VirtAddr, end_va: VirtAddr, port: usize) {
        //生成Permission
        let mut permission = MapPermission::from_bits((port as u8) << 1).unwrap();
        permission.set(MapPermission::U, true);
        let current = self.inner.exclusive_access().current_task;
        self.inner.exclusive_access().tasks[current].memory_set.insert_framed_area(start_va, end_va, permission);
    }
```
### 实现sys_munmap
1.sys_munmap(方法)
```rust
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
```
2.辅助函数release_area()，为MemorySet实现，用于检测区域是否存在映射并释放映射，被封装在TaskManager的check_map_then_unmap()方法中
```rust
    ///找到有关area并解除映射
    pub fn release_area(&mut self, start_vpn: VirtPageNum, end_vpn: VirtPageNum) -> isize {
        let mut flag = -1;
        for i in 0..self.areas.len() {
            if self.areas[i].match_range(start_vpn, end_vpn) {
                flag = 0;
                //释放
                self.areas[i].unmap(&mut self.page_table);
                self.areas.remove(i);
                break;
            }
        }
        flag
    }
```
## 实验结果
在os目录下运行如下命令。
```
make run BASE=0
```
可以通过如下测试用例：  
1.ch3_sleep  
2.ch3_sleep1  
3.ch3_taskinfo  
4.ch4_mmap0  
5.ch4_mmap1  
6.ch4_mmap2  
7.ch4_mmap3  
8.ch4_unmap2  
无法通过如下测试用例：  
1.ch4_unmap 【用例可以正常分配映射与解除映射，但在访存过程中由于缺页中断退出】 

## 实验总结
在本次实验中，我详细了解了虚存系统的运作原理与具体的代码实现，并在此基础上重写了sys_get_time与sys_task_info系统调用，知道了如何通过虚存地址与地址映射机制访问并修改程序空间中的变量。  
此外，我还为系统实现了将虚存空间映射到物理空间的方法与解除映射的方法，明白了为程序申请内存的过程、检查页表并分配物理内存的过程以及解除映射的过程，更对操作系统如何管理程序的页面以及程序的地址空间构成有了进一步的认识。    
在编写程序的过程中，我对rust语言的项目结构、所有权机制以及一系列方法与trait如copy_to()方法有了更深入的认识，受益匪浅