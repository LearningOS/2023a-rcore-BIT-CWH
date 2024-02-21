# ch8实验报告
## 实验内容
1.在“霸王龙”操作系统的基础上完成ch3、ch4、ch5的实验内容并通过测试用例。  
2.实现sys_link系统调用，实现硬连接。  
3.实现sys_unlink系统调用，完成链接解除。  
4.实现sys_state系统调用查询文件状态。
## 设计思路
### 完成sys_link系统调用
1.在Inode下新增link_num用与储存连接数，初始化为1。  
2.该系统调用新老目录名，首先进行参数检查：检查两个目录名是否相同。如果相同则为错误。  
3.在根目录下查询目标索引节点是否存在，如果存在根据新目录名为其新增目录。  
### 完成sys_unlink系统调用
1.在根目录下查找目标索引节点是否存在，并将剩余目录项保存到新的数组中待用。  
2.减少目标索引节点的连接数，如果连接数为0，将其删除。  
3.将剩余的目录项重新写入目录的数据节点中。
### 完成sys_state系统调用
1.为file trait新增方法state用于查询当前文件的状态。  
2.为OSInoide实现state方法。  
## 实验代码
### 完成sys_link系统调用
1.sys_link()系统调用
```rust
/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let old_name = translated_str(token, _old_name);
    let new_name = translated_str(token, _new_name);
    //判断是否映射同一文件
    if old_name.eq(new_name.as_str()) {
        return -1;
    }
    ROOT_INODE.linkat(&old_name, &new_name)
}
```
2.Inode的linkat()方法
```rust
    ///linkat
    pub fn linkat(&self, old_name: &str, new_name: &str) -> isize {
        //互斥访问fs
        let mut fs = self.fs.lock();
        //判断老索引节点是否存在
        let inode_id = self.read_disk_inode(|root_inode| {
            self.find_inode_id(old_name, root_inode)
        });
        if inode_id.is_none() {
            return -1;
        }
        let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id.unwrap());
        //修改引用计数
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, |inode: &mut DiskInode| {
                inode.link_num += 1;
            });
        //创建新目录项
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            self.increase_size( new_size as u32, root_inode, &mut fs);
            let dirent = DirEntry::new(new_name, inode_id.unwrap());
            root_inode.write_at(
                file_count * DIRENT_SZ, 
                &dirent.as_bytes(),
                &self.block_device)
        });
        block_cache_sync_all();
        0
    }
```
### 完成sys_unlink系统调用
1.sys_unlink()系统调用
```rust
/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let name = translated_str(token, _name);
    ROOT_INODE.unlinkat(&name)
}
```
2.Inode的unlinkat()方法
```rust
///unlinkat
    pub fn unlinkat(&self, name: &str) -> isize{
        let mut fs = self.fs.lock();
        let mut inode_id: Option<u32> = None;
        let mut v: Vec<DirEntry> = Vec::new();
    
        // 获取目标dirent
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    root_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if dirent.name() != name {
                    v.push(dirent);
                } else {
                    inode_id = Some(dirent.inode_id());
                }
            }
        });
    
        // 重置目录块
        self.modify_disk_inode(|root_inode| {
            let size = root_inode.size;
            let data_blocks_dealloc = root_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
            self.increase_size((v.len() * DIRENT_SZ) as u32, root_inode, &mut fs);
            for (i, dirent) in v.iter().enumerate() {
                root_inode.write_at(i * DIRENT_SZ, dirent.as_bytes(), &self.block_device);
            }
        });
        if inode_id.is_none() {
            return -1;
        }
        //获取原节点信息
        let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id.unwrap());
        //修改原disknode
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, |n: &mut DiskInode| {
                n.link_num -= 1;
                if n.link_num == 0 {
                    let size = n.size;
                    let data_blocks_dealloc = n.clear_size(&self.block_device);
                    assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
                    for data_block in data_blocks_dealloc.into_iter() {
                        fs.dealloc_data(data_block);
                    }
                }
            });
        block_cache_sync_all();
        0
    }
```
### 完成sys_state系统调用
1.为OSInode实现state()方法
```rust
    fn state(&self, st: &mut super::Stat) -> isize {
        let inner = self.inner.exclusive_access();
        inner.inode.read_disk_inode(|disk_inode| {
            st.mode = match disk_inode.type_ {
                DiskInodeType::File => StatMode::FILE,
                DiskInodeType::Directory => StatMode::DIR,
            };
            st.nlink = disk_inode.link_num as u32;
        });
        0
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
14.ch6.file0  
15.ch6.file1  
16.ch6.file2  
17.ch6.file3  
其中，ch5_stride中完成进程的优先级顺序为8、7、6、5、9、10，运行时间与优先级比例分别为1798900、1808457、1802266、1780960、1822577、1820920满足通过的要求：最大值比最小值小于1.5。
