#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! eastmoneyx 热路径：CSV → 值对象的离线解析。
//!
//! 只压测本库自有代码（表头校验、数值解析、身份去重、完整性校验），
//! 不含任何网络或文件 I/O。`cargo bench` 需配合 `harness = false`。

use std::hint::black_box;
use std::time::Instant;

use eastmoneyx::{parse_eastmoney_observations, EastMoneyMoneySupply};

/// 构造 `rows` 条货币供应量数据行（列名与 `docs/标准.md` §3 一致）。
fn synthetic_csv(rows: usize) -> String {
    let mut csv = String::from("series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n");
    for index in 0..rows {
        let year = 2000 + index / 12;
        let month = index % 12 + 1;
        csv.push_str(&format!(
            "money_supply,{year:04}-{month:02},110.5,660.1,3050.2,9.1,6.2,8.0,亿元\n"
        ));
    }
    csv
}

fn main() {
    const ITERS: u32 = 200;
    let csv = synthetic_csv(64);

    // 预热：让分配器与布局稳定，避免把首次成本算进基线。
    let warm: Vec<EastMoneyMoneySupply> = parse_eastmoney_observations(&csv).expect("预热解析");
    black_box(warm.len());

    let start = Instant::now();
    let mut observations = 0usize;
    for _ in 0..ITERS {
        let parsed = parse_eastmoney_observations(black_box(&csv)).expect("解析");
        observations = observations.wrapping_add(parsed.len());
        black_box(&parsed);
    }
    let elapsed = start.elapsed();
    println!(
        "bench_eastmoneyx_csv_parse: rows={} iters={ITERS} total={elapsed:?} per_iter={:?} observations={}",
        csv.lines().count() - 1,
        elapsed / ITERS,
        black_box(observations)
    );
}
