//! File and filesystem-related syscalls
use core::mem::size_of;

use crate::fs::{open_file,get_root_inode,OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    // trace!(
    //     "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
    //     current_task().unwrap().pid.0
    // );
    // -1
    // 获取当前任务
    let task = current_task().unwrap();

    let token = task.get_user_token();

    let inner = task.inner_exclusive_access();
    
    // 检查文件描述符是否有效
    if fd >= inner.fd_table.len() {
        return -1;
    }
    
    if let Some(file) = &inner.fd_table[fd] {
        // 获取文件的元数据
        if let Some(inode) = file.get_stat() {
            debug!("file state 0: {:?}", inode);            
            // 将 Stat 结构体写入用户空间
            let state_bytes = unsafe {
                core::slice::from_raw_parts(
                    (&inode as *const Stat) as *const u8,
                    size_of::<Stat>(),
                )
            };
            let data_ptr = st as *mut u8;
            // 简单地按字节逐个拷贝，效率较低
            let v = translated_byte_buffer(token, data_ptr, size_of::<Stat>());
            let flat_dest_iter = v.into_iter().flatten();
            for (dest_byte, src_byte) in flat_dest_iter.zip(state_bytes.iter()) {
                *dest_byte = *src_byte;
            }
            return 0;
        }
        // 如果文件不支持获取状态，返回错误
        return -1;
    } else {
        // 文件描述符无效
        -1
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    // trace!(
    //     "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
    //     current_task().unwrap().pid.0
    // );
    // -1
    trace!(
        "kernel:pid[{}] sys_linkat",
        current_task().unwrap().pid.0
    );

    // 获取当前用户的token
    let token = current_user_token();
    
    // 从用户空间读取文件路径
    let old_path = translated_str(token, old_name);
    let new_path = translated_str(token, new_name);
    
    // 检查是否链接同名文件
    if old_path == new_path {
        return -1;
    }
    
    // 尝试打开原文件，确认它存在
    if let Some(_) = open_file(old_path.as_str(), OpenFlags::RDONLY) {

        // 获取文件系统的根目录
        let fs_root = get_root_inode();
        
        // 尝试创建硬链接
        match fs_root.link_file(old_path.as_str(), new_path.as_str()) {
            Ok(links) => {
                links as isize
            },  // 成功创建链接,返回链接数
            Err(_) => -1 // 创建链接失败
        }
    } else {
        // 原文件不存在
        -1
    }
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat",
        current_task().unwrap().pid.0
    );
    
    let token = current_user_token();
    
    // 从用户空间读取文件路径
    let path_str = translated_str(token, name);
    
    // 获取文件系统的根目录
    let fs_root = get_root_inode();
    // 尝试删除文件
    match fs_root.unlink(path_str.as_str()) {
        Ok(()) => 0,  // 成功删除文件
        Err(()) => -1 // 删除文件失败，可能是文件不存在
    }
}
