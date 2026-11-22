use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Frame;
use tunasync_protocol::{MirrorStatus, SyncStatus};

use super::{
    format_command, localized, redact_url, safe_display_text, App, Overlay, Pending, View,
};
use crate::{fmt_status, fmt_time};

pub(super) fn render(frame: &mut Frame<'_>, app: &mut App, manager: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(4),
        ])
        .split(frame.area());

    render_header(frame, app, manager, chunks[0]);
    match app.view {
        View::Mirrors => render_jobs(frame, app, chunks[1]),
        View::Workers => render_workers(frame, app, chunks[1]),
    }
    render_detail(frame, app, chunks[2]);
    render_footer(frame, app, chunks[3]);

    if let Some(overlay) = &app.overlay {
        render_overlay(frame, overlay, app.zh);
    }
    if let Some(pending) = &app.pending {
        render_confirmation(frame, pending, app.zh);
    }
}

fn render_header(frame: &mut Frame<'_>, app: &App, manager: &str, area: Rect) {
    let manager = safe_display_text(manager, 160);
    let refreshed = app
        .last_refresh
        .map(|time| time.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "--:--:--".to_owned());
    let activity = if app.refreshing {
        localized(app.zh, "refreshing", "刷新中")
    } else if app.command_in_flight {
        localized(app.zh, "command pending", "命令执行中")
    } else {
        localized(app.zh, "ready", "就绪")
    };
    let (total, syncing, failed, stale) = app.counts();
    let maint = match app.maintenance {
        Some(true) => localized(app.zh, "MAINT", "维护"),
        Some(false) => localized(app.zh, "normal", "正常"),
        None => "-",
    };
    let search = if app.search_query.is_empty() && !app.search_input {
        String::new()
    } else if app.zh {
        format!(
            " | 搜索: {}{}",
            app.search_query,
            if app.search_input { "_" } else { "" }
        )
    } else {
        format!(
            " | search: {}{}",
            app.search_query,
            if app.search_input { "_" } else { "" }
        )
    };
    let title = if app.zh {
        format!(
            " tunasynctl | {manager} | {activity} | 更新 {refreshed} | 共{total} 同步{syncing} 失败{failed} stale{stale} | {maint} | Worker: {} | 状态: {}{search} ",
            safe_display_text(app.worker_filter_label(), 80),
            app.status_filter_label()
        )
    } else {
        format!(
            " tunasynctl | {manager} | {activity} | updated {refreshed} | total {total} syncing {syncing} failed {failed} stale {stale} | {maint} | worker: {} | status: {}{search} ",
            safe_display_text(app.worker_filter_label(), 80),
            app.status_filter_label()
        )
    };
    frame.render_widget(
        Paragraph::new(title)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_jobs(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let rows = app.visible_jobs().into_iter().map(|job| {
        Row::new(vec![
            safe_display_text(&job.name, 120),
            safe_display_text(&job.worker, 120),
            fmt_status(&job.status),
            fmt_time(&job.last_update),
            display_or_dash(&safe_display_text(&job.size, 60)),
        ])
        .style(status_style(job.status))
    });
    let headings = if app.zh {
        ["镜像", "Worker", "状态", "最后更新", "大小"]
    } else {
        ["Mirror", "Worker", "Status", "Last update", "Size"]
    };
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(28),
            Constraint::Percentage(20),
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Min(8),
        ],
    )
    .header(
        Row::new(headings).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().borders(Borders::ALL).title(if app.zh {
        format!(" 镜像 ({}) ", app.visible_job_indices().len())
    } else {
        format!(" Mirrors ({}) ", app.visible_job_indices().len())
    }))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("› ");
    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_workers(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let rows = app.worker_rows.iter().map(|worker| {
        let jobs = app.jobs.iter().filter(|job| job.worker == worker.id);
        let total = jobs.clone().count();
        let failed = jobs
            .clone()
            .filter(|job| job.status == SyncStatus::Failed)
            .count();
        let syncing = jobs
            .filter(|job| matches!(job.status, SyncStatus::Syncing | SyncStatus::PreSyncing))
            .count();
        Row::new(vec![
            safe_display_text(&worker.id, 120),
            fmt_time(&worker.last_online),
            total.to_string(),
            failed.to_string(),
            syncing.to_string(),
            safe_display_text(&redact_url(&worker.url), 160),
        ])
    });
    let headings = if app.zh {
        ["Worker", "最后在线", "镜像", "失败", "同步中", "URL"]
    } else {
        [
            "Worker",
            "Last online",
            "Mirrors",
            "Failed",
            "Syncing",
            "URL",
        ]
    };
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(18),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(12),
        ],
    )
    .header(
        Row::new(headings).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().borders(Borders::ALL).title(if app.zh {
        format!(" Worker ({}) ", app.worker_rows.len())
    } else {
        format!(" Workers ({}) ", app.worker_rows.len())
    }))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("› ");
    frame.render_stateful_widget(table, area, &mut app.worker_table_state);
}

fn render_detail(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let text = app.selected_job().map_or_else(
        || {
            Text::from(localized(
                app.zh,
                "No mirror matches the current filters.",
                "当前筛选条件下没有镜像。",
            ))
        },
        |job| Text::from(detail_lines(job, app.zh)),
    );
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(localized(app.zh, " Detail ", " 详情 ")),
        ),
        area,
    );
}

fn detail_lines(job: &MirrorStatus, zh: bool) -> Vec<Line<'static>> {
    let upstream = redact_url(&job.upstream);
    let name = safe_display_text(&job.name, 120);
    let worker = safe_display_text(&job.worker, 120);
    let upstream = safe_display_text(&upstream, 500);
    let error = safe_display_text(&job.error_msg, 1000);
    let master = yes_no(zh, job.is_master);
    let stale = yes_no(zh, job.stale);
    if zh {
        vec![
            Line::from(format!(
                "名称: {}    Worker: {}    状态: {}",
                name,
                worker,
                fmt_status(&job.status)
            )),
            Line::from(format!(
                "主节点: {master}    Stale: {stale}    连续失败: {}",
                job.consecutive_failures
            )),
            Line::from(format!(
                "开始: {}    结束: {}",
                fmt_time(&job.last_started),
                fmt_time(&job.last_ended)
            )),
            Line::from(format!(
                "下次运行: {}    最近传输: {}",
                fmt_time(&job.scheduled),
                format_bytes(job.last_transferred_bytes)
            )),
            Line::from(format!("Upstream: {}", display_or_dash(&upstream))),
            Line::from(format!("错误: {}", display_or_dash(&error))),
        ]
    } else {
        vec![
            Line::from(format!(
                "Name: {}    Worker: {}    Status: {}",
                name,
                worker,
                fmt_status(&job.status)
            )),
            Line::from(format!(
                "Master: {master}    Stale: {stale}    Consecutive failures: {}",
                job.consecutive_failures
            )),
            Line::from(format!(
                "Started: {}    Ended: {}",
                fmt_time(&job.last_started),
                fmt_time(&job.last_ended)
            )),
            Line::from(format!(
                "Next run: {}    Last transferred: {}",
                fmt_time(&job.scheduled),
                format_bytes(job.last_transferred_bytes)
            )),
            Line::from(format!("Upstream: {}", display_or_dash(&upstream))),
            Line::from(format!("Error: {}", display_or_dash(&error))),
        ]
    }
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let help = localized(
        app.zh,
        "j/k Tab /  w s r  a start x stop R restart d disable  m maint  L reload  h history l logs  q quit",
        "j/k Tab /  w s r  a 启动 x 停止 R 重启 d 禁用  m 维护  L 重载  h 历史 l 日志  q 退出",
    );
    let text = Text::from(vec![
        Line::from(Span::styled(
            safe_display_text(&app.status_message, 500),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(help),
    ]);
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_confirmation(frame: &mut Frame<'_>, pending: &Pending, zh: bool) {
    let area = centered_rect(68, 7, frame.area());
    let prompt = match pending {
        Pending::Job(command) => {
            let mirror = safe_display_text(&command.mirror_id, 120);
            let worker = safe_display_text(&command.worker_id, 120);
            let action = format_command(command.cmd, zh);
            if zh {
                format!("确认对镜像 {mirror:?}（Worker {worker:?}）执行{action}？\n\nEnter/y 确认，Esc/n 取消")
            } else {
                format!("Run {action} on mirror {mirror:?} (worker {worker:?})?\n\nEnter/y confirms, Esc/n cancels")
            }
        }
        Pending::Reload { worker } => {
            let worker = safe_display_text(worker, 120);
            if zh {
                format!("确认让 Worker {worker:?} 重新加载配置？\n\nEnter/y 确认，Esc/n 取消")
            } else {
                format!("Reload config on worker {worker:?}?\n\nEnter/y confirms, Esc/n cancels")
            }
        }
        Pending::Maintenance { enable } => {
            if zh {
                if *enable {
                    "确认启用维护模式？维护期间 manager 会拒绝新的同步启动。\n\nEnter/y 确认，Esc/n 取消".to_owned()
                } else {
                    "确认关闭维护模式并恢复正常操作？\n\nEnter/y 确认，Esc/n 取消".to_owned()
                }
            } else if *enable {
                "Enable maintenance mode? New sync starts will be rejected.\n\nEnter/y confirms, Esc/n cancels".to_owned()
            } else {
                "Disable maintenance mode and resume normal operation?\n\nEnter/y confirms, Esc/n cancels".to_owned()
            }
        }
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(prompt)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(localized(zh, " Confirm action ", " 确认操作 ")),
            ),
        area,
    );
}

fn render_overlay(frame: &mut Frame<'_>, overlay: &Overlay, zh: bool) {
    let area = centered_rect(86, 16, frame.area());
    let title = match overlay.kind {
        super::OverlayKind::History => {
            if zh {
                format!(" 历史 {} ", safe_display_text(&overlay.title, 80))
            } else {
                format!(" History {} ", safe_display_text(&overlay.title, 80))
            }
        }
        super::OverlayKind::Logs => {
            if zh {
                format!(" 日志 {} ", safe_display_text(&overlay.title, 80))
            } else {
                format!(" Logs {} ", safe_display_text(&overlay.title, 80))
            }
        }
    };
    let body = if overlay.lines.is_empty() {
        localized(zh, "No output.", "没有输出。").to_owned()
    } else {
        overlay
            .lines
            .iter()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(body).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        ),
        area,
    );
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical_margin = area.height.saturating_sub(height) / 2;
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_margin),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    let horizontal_margin = 100u16.saturating_sub(percent_x) / 2;
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(horizontal_margin),
            Constraint::Percentage(percent_x),
            Constraint::Percentage(horizontal_margin),
        ])
        .split(vertical[1])[1]
}

fn status_style(status: SyncStatus) -> Style {
    let color = match status {
        SyncStatus::Success => Color::Green,
        SyncStatus::Failed => Color::Red,
        SyncStatus::Syncing | SyncStatus::PreSyncing => Color::Cyan,
        SyncStatus::Paused => Color::Yellow,
        SyncStatus::Disabled => Color::DarkGray,
        SyncStatus::None => Color::White,
    };
    Style::default().fg(color)
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "-".to_owned();
    }
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn display_or_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_owned()
    } else {
        value.to_owned()
    }
}

fn yes_no(zh: bool, value: bool) -> &'static str {
    match (zh, value) {
        (true, true) => "是",
        (true, false) => "否",
        (false, true) => "yes",
        (false, false) => "no",
    }
}
