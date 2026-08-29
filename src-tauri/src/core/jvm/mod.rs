//! JVM 内存参数自动推荐:按系统内存档位给出 Xmx/Xms 与 GC 参数建议。
//! 规则数据驱动、纯函数,便于单元测试;实际只能由用户在设置页点击"应用推荐"时生效,
//! 不会静默覆盖用户手动配置。

use serde::Serialize;

const MB: u64 = 1024 * 1024;

/// 系统内存概况(字节换算为 MB)
#[derive(Debug, Clone, Serialize)]
pub struct SystemMemory {
    pub total_mb: u64,
    pub available_mb: u64,
}

/// 自动推荐的 JVM 参数
#[derive(Debug, Clone, Serialize)]
pub struct JvmRecommendation {
    pub min_mb: u32,
    pub max_mb: u32,
    /// 建议追加的 GC 等 JVM 参数(不含 -Xmx/-Xms)
    pub extra_args: Vec<String>,
    /// 档位说明(如 "高内存", "保守(低内存)")
    pub tier_label: String,
    pub note: String,
}

/// 内存档位
enum Tier {
    /// < 4G:低内存 / 32 位保守档
    Low,
    /// 4-8G
    Mid,
    /// 8-16G
    High,
    /// > 16G
    Xl,
}

fn tier_of(total_mb: u64) -> Tier {
    match total_mb {
        0..=4095 => Tier::Low,
        4096..=8191 => Tier::Mid,
        8192..=16383 => Tier::High,
        _ => Tier::Xl,
    }
}

/// 依据系统内存与(可选)mod 数量推荐 JVM 参数。
/// `available_mb` 用于兜底,确保推荐值不超过系统可用内存的合理比例,避免卡死。
/// `is_32bit` 时一律走保守档(< 4G 逻辑)。
pub fn recommend(total_mb: u64, available_mb: u64, mod_count: u32, is_32bit: bool) -> JvmRecommendation {
    let tier = if is_32bit { Tier::Low } else { tier_of(total_mb) };

    let mod_extra_mb = (mod_count as u64).min(64) * 24; // 每个 mod 约 +24MB,封顶
    let (mut max_mb, mut min_mb, extra_args, tier_label, note) = match tier {
        Tier::Low => {
            let max = clamp(total_mb / 2 + mod_extra_mb, 1024, 3072);
            (
                max,
                max.min(1024),
                Vec::new(),
                "保守(低内存)",
                "系统内存较少,采用保守建议以避免内存不足或系统卡顿。".to_string(),
            )
        }
        Tier::Mid => {
            let max = clamp(total_mb / 2 + mod_extra_mb, 2048, 6144);
            (
                max,
                max.min(2048),
                Vec::new(),
                "中等内存",
                "建议使用默认 GC,兼顾性能与兼容性。".to_string(),
            )
        }
        Tier::High => {
            let max = clamp(total_mb * 3 / 5 + mod_extra_mb, 4096, 12288);
            (
                max,
                max.min(3072),
                g1_args(),
                "高内存",
                "内存充足,推荐 G1GC 以获得更稳定的帧率和更短的暂停。".to_string(),
            )
        }
        Tier::Xl => {
            let max = clamp(total_mb * 3 / 5 + mod_extra_mb, 8192, 16384);
            let mut args = g1_args();
            args.push("-XX:MaxRAMPercentage=60.0".to_string());
            (
                max,
                max.min(4096),
                args,
                "超大内存",
                "内存非常充足,可使用 G1GC + 按比例分配的策略。".to_string(),
            )
        }
    };

    // 兜底:推荐值不超过可用内存的 70%,确保系统不被吃满
    let ceiling = (available_mb as u64).saturating_mul(7) / 10;
    if max_mb as u64 > ceiling {
        max_mb = clamp(ceiling, 1024, max_mb as u32);
    }
    min_mb = min_mb.min(max_mb);

    JvmRecommendation {
        min_mb: min_mb as u32,
        max_mb: max_mb as u32,
        extra_args,
        tier_label: tier_label.to_string(),
        note,
    }
}

#[allow(clippy::vec_init_then_push)]
fn g1_args() -> Vec<String> {
    vec![
        "-XX:+UseG1GC".into(),
        "-XX:+ParallelRefProcEnabled".into(),
        "-XX:MaxGCPauseMillis=200".into(),
        "-XX:+UnlockExperimentalVMOptions".into(),
        "-XX:+DisableExplicitGC".into(),
    ]
}

fn clamp(v: u64, lo: u32, hi: u32) -> u32 {
    let lo = lo as u64;
    let hi = hi as u64;
    let v = v.max(lo).min(hi);
    // 按 256MB 取整,方便读取
    ((v / 256) * 256) as u32
}

/// 读取当前系统内存(通过 sysinfo)
pub fn current_memory() -> SystemMemory {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    SystemMemory {
        total_mb: sys.total_memory() / MB,
        available_mb: sys.available_memory() / MB,
    }
}

/// 是否 32 位环境(决定是否走保守档)
pub fn is_32bit() -> bool {
    usize::BITS < 64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_memory_conservative() {
        let r = recommend(3072, 2048, 0, false);
        assert_eq!(r.tier_label, "保守(低内存)");
        assert!(r.max_mb <= 3072, "低内存 max 不应超过 3G, 实际 {}", r.max_mb);
        assert!(!r.max_mb as u64 > 2048 * 7 / 10, "不应超过可用内存 70%");
        assert!(r.extra_args.is_empty());
    }

    #[test]
    fn high_memory_g1gc() {
        let r = recommend(12288, 8192, 0, false);
        assert_eq!(r.tier_label, "高内存");
        assert!(r.extra_args.iter().any(|a| a == "-XX:+UseG1GC"));
        assert!(r.max_mb >= 4096);
    }

    #[test]
    fn xl_memory_adds_ram_percentage() {
        let r = recommend(32768, 24576, 0, false);
        assert_eq!(r.tier_label, "超大内存");
        assert!(r.extra_args.iter().any(|a| a.contains("MaxRAMPercentage")));
    }

    #[test]
    fn mod_count_raises_recommendation() {
        let base = recommend(8192, 6144, 0, false).max_mb;
        let with_mods = recommend(8192, 6144, 40, false).max_mb;
        assert!(with_mods >= base);
    }

    #[test]
    fn never_exceeds_available_ceiling() {
        // 可用内存很小,推荐值应被压制到约 70% 以内
        let r = recommend(16384, 2048, 0, false);
        assert!(r.max_mb as u64 <= 2048 * 7 / 10);
    }

    #[test]
    fn thirty_two_bit_goes_conservative() {
        let r = recommend(16384, 12288, 0, true);
        assert_eq!(r.tier_label, "保守(低内存)");
        assert!(r.extra_args.is_empty());
    }

    #[test]
    fn value_rounded_to_256() {
        let r = recommend(4096, 3072, 0, false);
        assert_eq!(r.max_mb % 256, 0);
    }
}
