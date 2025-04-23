//! 死锁检测算法实现

use alloc::vec;
use alloc::vec::Vec;
use crate::task::{current_process, current_task};

use super::mutex::MutexType;

/// 死锁检测结果
#[derive(PartialEq)]
pub enum DeadlockResult {
    /// 没有死锁
    Safe,
    /// 检测到死锁
    Deadlock,
}

/// 检测 mutex 死锁
pub fn detect_mutex_deadlock(mutex_id: usize) -> DeadlockResult {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    
    // 如果没有启用死锁检测，直接返回安全
    if !process_inner.enable_deadlock_detect {
        return DeadlockResult::Safe;
    }
    
    // 获取当前线程ID
    let current_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    
    // 构建资源分配图
    let mutex_count = process_inner.mutex_list.len();
    let task_count = process_inner.tasks.len();
    debug!("mutex_count 2: {}", mutex_count);
    // 可用资源向量 Available
    // 此部分由AI协助完成>>>>>>>
    let mut available = Vec::with_capacity(mutex_count);
    for i in 0..mutex_count {
        if let Some(mutex) = &process_inner.mutex_list[i] {
            // 对于MutexSpin和MutexBlocking，我们需要检查它们是否被锁定
            let is_available = match mutex.as_ref() {
                mutex_spin if (mutex_spin.get_type() == MutexType::Spin) => {
                    let mutex_spin = unsafe { &*(mutex_spin as *const dyn super::Mutex as *const super::MutexSpin) };
                    !*mutex_spin.locked.exclusive_access()
                },
                mutex_blocking if (mutex_blocking.get_type() == MutexType::Blocking) => {
                    let mutex_blocking = unsafe { &*(mutex_blocking as *const dyn super::Mutex as *const super::MutexBlocking) };
                    !mutex_blocking.inner.exclusive_access().locked
                },
                _ => true, // 默认情况下假设可用
            };
            available.push(if is_available { 1 } else { 0 });
        } else {
            available.push(0); // 不存在的互斥锁视为不可用
        }
    }
    // <<<<<<<<此部分由AI协助完成
    debug!("mutex_count 3: {}", mutex_count);
    
    // 分配矩阵 Allocation
    let  allocation = vec![vec![0; mutex_count]; task_count];
    
    // 需求矩阵 Need
    let mut need = vec![vec![0; mutex_count]; task_count];
    
    // 当前线程需要获取的互斥锁
    need[current_tid][mutex_id] = 1;
    
    // 工作向量 Work
    let mut work = available.clone();
    
    // 结束向量 Finish
    let mut finish = vec![false; task_count];
    
    // 银行家算法
    loop {
        let mut found = false;
        
        for i in 0..task_count {
            if !finish[i] {
                let mut can_allocate = true;
                for j in 0..mutex_count {
                    if need[i][j] > work[j] {
                        can_allocate = false;
                        break;
                    }
                }
                
                if can_allocate {
                    found = true;
                    finish[i] = true;
                    
                    for j in 0..mutex_count {
                        work[j] += allocation[i][j];
                    }
                    
                    break;
                }
            }
        }
        
        if !found {
            break;
        }
    }
    
    // 检查是否所有线程都能完成
    for i in 0..task_count {
        if !finish[i] {
            return DeadlockResult::Deadlock;
        }
    }
    
    DeadlockResult::Safe
}

/// 检测 semaphore 死锁
pub fn detect_semaphore_deadlock(sem_id: usize) -> DeadlockResult {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    debug!("semaphore deadlock 0");
    // 如果没有启用死锁检测，直接返回安全
    if !process_inner.enable_deadlock_detect {
        debug!("kernel: Semaphore deadlock detection disabled");
        return DeadlockResult::Safe;
    }
    
    // 获取当前线程ID
    let current_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    
    // 构建资源分配图
    let sem_count = process_inner.semaphore_list.len();
    let task_count = process_inner.tasks.len();
    
    // 可用资源向量 Available
    let mut available = Vec::with_capacity(sem_count);
    for i in 0..sem_count {
        if let Some(sem) = &process_inner.semaphore_list[i] {
            let count = sem.inner.exclusive_access().count;
            available.push(if count > 0 { count as usize } else { 0 });
        } else {
            debug!("kernel: Semaphore {} does not exist", i);
            available.push(0); // 不存在的信号量视为不可用
        }
    }
    
    // 分配矩阵 Allocation
    let allocation = vec![vec![0; sem_count]; task_count];
    
    // 需求矩阵 Need
    let mut need = vec![vec![0; sem_count]; task_count];
    
    // 当前线程需要获取的信号量
    need[current_tid][sem_id] = 1;
    
    // 工作向量 Work
    let mut work = available.clone();
    
    // 结束向量 Finish
    let mut finish = vec![false; task_count];
    
    // 银行家算法
    loop {
        let mut found = false;
        
        for i in 0..task_count {
            if !finish[i] {
                let mut can_allocate = true;
                for j in 0..sem_count {
                    if need[i][j] > work[j] {
                        can_allocate = false;
                        break;
                    }
                }
                
                if can_allocate {
                    found = true;
                    finish[i] = true;
                    
                    for j in 0..sem_count {
                        work[j] += allocation[i][j];
                    }
                    
                    break;
                }
            }
        }
        
        if !found {
            break;
        }
    }

    // 检查是否所有线程都能完成
    for i in 0..task_count {
        if !finish[i] {
            return DeadlockResult::Deadlock;
        }
    }
    
    DeadlockResult::Safe
}