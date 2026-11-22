pub mod cmd_provider;
pub mod rsync_provider;
pub mod two_stage_rsync_provider;

pub use cmd_provider::CmdProvider;
pub use rsync_provider::RsyncProvider;
pub use two_stage_rsync_provider::TwoStageRsyncProvider;

/// Merge success exit codes in the same order and with the same deduplication
/// semantics as the Go implementation in `newMirrorProvider`.
///
/// Order (first occurrence wins):
/// 1. global.dangerous_global_success_exit_codes
/// 2. mirror.success_exit_codes
/// 3. (rsync providers only) global.dangerous_global_rsync_success_exit_codes
/// 4. (rsync providers only) mirror.rsync_success_exit_codes
pub fn merge_success_exit_codes(
    global_success: &[i32],
    mirror_success: &[i32],
    global_rsync: &[i32],
    mirror_rsync: &[i32],
    is_rsync: bool,
) -> Vec<i32> {
    let mut codes = Vec::new();
    codes.extend_from_slice(global_success);
    codes.extend_from_slice(mirror_success);
    if is_rsync {
        codes.extend_from_slice(global_rsync);
        codes.extend_from_slice(mirror_rsync);
    }
    // Deduplicate while preserving first-seen order (matches Go).
    if codes.len() > 1 {
        let mut seen = std::collections::HashSet::new();
        codes.retain(|c| seen.insert(*c));
    }
    codes
}
