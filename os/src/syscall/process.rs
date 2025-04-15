//! Process management syscalls

use crate::config::PAGE_SIZE;
use crate::mm::{translated_byte_buffer, MapPermission, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, get_systrace, map_new_area,
    set_sys_trace, suspend_current_and_run_next, unmap_area,
};
use crate::timer::get_time_us;
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    // debug!("sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    // 将结构体转换为字节切片
    let time_val_bytes = unsafe {
        core::slice::from_raw_parts(
            (&time_val as *const TimeVal) as *const u8,
            size_of::<TimeVal>(),
        )
    };
    let token = current_user_token();
    let data_ptr = ts as *mut u8;
    // 简单地按字节逐个拷贝，效率较低
    let v = translated_byte_buffer(token, data_ptr, size_of::<TimeVal>());
    let flat_dest_iter = v.into_iter().flatten();
    for (dest_byte, src_byte) in flat_dest_iter.zip(time_val_bytes.iter()) {
        *dest_byte = *src_byte;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    match trace_request {
        0 => {
            debug!("sys_get_trace 0");
            // 读取当前任务 id 地址处一个字节的无符号整数值
            let addr = id as *const u8;
            let pt = PageTable::from_token(current_user_token());
            let start_va = VirtAddr::from(addr as usize);
            let vpn = start_va.floor();
            let Some(pte) = pt.translate(vpn) else {
                return -1;
            };
            debug!(
                "*****is_valid: {}, readable: {}, user_accessible: {}",
                pte.is_valid(),
                pte.readable(),
                pte.user_accessible()
            );
            if !pte.is_valid() || !pte.readable() || !pte.user_accessible() {
                return -1;
            }

            let buffers =
                translated_byte_buffer(current_user_token(), id as *const u8, size_of::<usize>());

            if let Some(value) = buffers.into_iter().flatten().next() {
                // 将第一个元素作为 isize 类型返回
                return *value as isize;
            }
            -1
        }
        1 => {
            debug!("sys_get_trace 1");
            // 写入 data 到该用户程序 id 地址处
            let data_byte = (data & 0xff) as u8; // 只取最低字节
            let addr = id as *mut u8;
            
            // 检查地址是否有效且可写
            let pt = PageTable::from_token(current_user_token());
            let start_va = VirtAddr::from(addr as usize);
            let vpn = start_va.floor();
            let Some(pte) = pt.translate(vpn) else {
                return -1;
            };
            
            debug!(
                "is_valid: {}, writable: {}, user_accessible: {}",
                pte.is_valid(),
                pte.writable(),
                pte.user_accessible()
            );
            
            // 检查页表项权限
            if !pte.is_valid() || !pte.writable() || !pte.user_accessible() {
                return -1;
            }
            
            // 写入数据
            let buffers = translated_byte_buffer(current_user_token(), addr, 1);
            if let Some(buffer) = buffers.into_iter().next() {
                if let Some(dest) = buffer.get_mut(0) {
                    *dest = data_byte;
                    return 0;
                }
            }
            -1
        }
        2 => {
            debug!("sys_get_trace 2");
            get_systrace(id)
        }
        _ => {
            -1
        }
    }
}

pub fn sys_trace_set(id: usize) {
    set_sys_trace(id);
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    debug!("sys_mmap start:{}  len:{} port:{}", start, len, port);
    if start % PAGE_SIZE != 0 || port & !0x7 != 0 || port & 0x7 == 0 || start % 4096 != 0 {
        return -1;
    }
    let mut map_permission = MapPermission::empty();
    if port & 0x1 != 0 {
        map_permission |= MapPermission::R;
    }
    if port & 0x2 != 0 {
        map_permission |= MapPermission::W;
    }
    if port & 0x4 != 0 {
        map_permission |= MapPermission::X;
    }
    map_permission.insert(MapPermission::U);

    map_new_area(start, len, map_permission).unwrap_or(-1)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    debug!("sys_munmap");
    if start % PAGE_SIZE != 0 || len == 0 || len % PAGE_SIZE != 0 {
        return -1;
    }
    unmap_area(start, len).unwrap_or(-1)
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
