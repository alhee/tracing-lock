use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::Instant;

macro_rules! log_call_info {
    () => {{
        let location = std::panic::Location::caller();
        let thread_name = std::thread::current()
            .name()
            .unwrap_or("unknown")
            .to_string();
        println!(
            "Function '{}' called at {}:{} on thread {}",
            std::any::type_name::<fn()>(),
            location.file(),
            location.line(),
            thread_name
        );
    }};
}

struct TokioRwLockTrace<T> {
    inner: Arc<RwLock<T>>,
}
impl<T> TokioRwLockTrace<T> {
    // 기존의 RwLock을 감싸는 새로운 생성자
    pub fn from(inner: Arc<RwLock<T>>) -> Self {
        TokioRwLockTrace { inner }
    }

    fn new(value: T) -> Self {
        TokioRwLockTrace {
            inner: Arc::new(RwLock::new(value)),
        }
    }
    pub async fn read(&self) -> LoggingRwLockReadGuard<'_, T> {
        log_call_info!();
        let guard = self.inner.read().await;
        LoggingRwLockReadGuard {
            guard,
            start_time: Instant::now(),
        }
    }

    pub async fn write(&self) -> LoggingRwLockWriteGuard<'_, T> {
        log_call_info!();
        let guard = self.inner.write().await;
        LoggingRwLockWriteGuard {
            guard,
            start_time: Instant::now(),
        }
    }
}

/**
 * * Deref 및 DerefMut 트레이트 구현
 *   Deref와 DerefMut 트레이트를 구현하면, 해당 구조체가 감싸고 있는 타입의 메서드에 자동으로 접근할 수 있다. (Rust의 자동 참조 역참조(dereferencing) 기능)
 *   `Deref` 트레이트는 `*` 연산자를 오버로딩하는데 사용된다 .* 연산자를 사용하여 대상 객체를 불변 참조로 변환
 *   `DerefMut` 트레이트는 `*mut` 연산자를 오버로딩하는데 사용된다. *mut 연산자를 사용하여 대상 객체를 가변 참조로 변환
 * * 메서드 탐색
 *   - 메서드 호출 시 컴파일러는 해당 메서드가 현재 타입에 존재하는지 확인.
 *   - 존재하지 않으면, Deref 또는 DerefMut를 통해 반환된 타입에서 메서드를 탐색
 * * 락을 획득하고 해제하는 시점을 정확히 로그에 기록하려면, RwLockWriteGuard와 RwLockReadGuard의 드롭 시점도 추적해야 한다.
 **/

pub struct LoggingRwLockReadGuard<'a, T> {
    guard: RwLockReadGuard<'a, T>,
    start_time: Instant,
}

impl<'a, T> Deref for LoggingRwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> Drop for LoggingRwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        println!("Read lock released. Duration: {:?}", duration);
        print_info();
    }
}

pub struct LoggingRwLockWriteGuard<'a, T> {
    guard: RwLockWriteGuard<'a, T>,
    start_time: Instant,
}

impl<'a, T> Deref for LoggingRwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T> DerefMut for LoggingRwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<'a, T> Drop for LoggingRwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        println!("Write lock released. Duration: {:?}", duration);
        print_info();
    }
}

// Deref 및 DerefMut 트레이트 구현
impl<T> Deref for TokioRwLockTrace<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for TokioRwLockTrace<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.inner).expect("Failed to get mutable reference")
    }
}

#[track_caller]
fn print_info() {
    let location = std::panic::Location::caller();
    let thread_name = std::thread::current()
        .name()
        .unwrap_or("unknown")
        .to_string();
    println!(
        "Function '{}' called at {}:{} on thread {}",
        std::any::type_name::<fn()>(),
        location.file(),
        location.line(),
        thread_name
    );
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn() {
        let rw_lock = Arc::new(RwLock::new(5));
        let logging_lock = TokioRwLockTrace::from(rw_lock.clone());

        let _ = tokio::spawn(async move {
            {
                let read_guard = logging_lock.read().await;
                println!("Read value: {}", *read_guard);
            }

            {
                let mut write_guard = logging_lock.write().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                *write_guard += 1;
                println!("Updated value: {}", *write_guard);
            }
        })
        .await;
    }
}
