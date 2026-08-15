use std::time::{SystemTime, UNIX_EPOCH};

/// 获取当前时间戳。
///
/// 返回自 UNIX 纪元以来经过的毫秒数。
///
/// # 返回值
///
/// 当前时间的毫秒时间戳。
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_is_positive_and_monotonic() {
        let t1 = now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let t2 = now();
        assert!(t1 > 0);
        assert!(t2 >= t1);
    }
}
