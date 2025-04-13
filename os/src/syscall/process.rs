//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{translated_byte_buffer, MapPermission};
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, map_new_area,
    suspend_current_and_run_next, unmap_area,
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
    trace!("kernel: sys_get_time");
    debug!("sys_get_time");
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
    // let mut remaining_bytes = time_val_bytes;
    // for slice in v {
    //     let copy_len = slice.len().min(remaining_bytes.len());
    //     slice[0..copy_len].copy_from_slice(&remaining_bytes[0..copy_len]);
    //     remaining_bytes = &remaining_bytes[copy_len..];
    // }
    // 0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if start % PAGE_SIZE != 0 || port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    let mut port = MapPermission::from_bits(port as u8).unwrap();
    port.insert(MapPermission::U);
    map_new_area(start, len, port).unwrap_or(-1);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    unmap_area(start, len).unwrap_or(-1);
    // TASK_MANAGER.map_new_area
    -1
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
