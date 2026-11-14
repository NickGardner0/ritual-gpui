use crate::time::civil;

/// Human-friendly modification time ("今天 14:03", "昨天", "10月3日").
pub fn format_time(secs: u64) -> String {
    let now = crate::time::now();
    let (y, mo, d, h, mi) = civil(secs);
    let (ny, nmo, nd, _, _) = civil(now);
    let (yy, ymo, yd, _, _) = civil(now.saturating_sub(86_400));
    if (y, mo, d) == (ny, nmo, nd) {
        crate::localized_format!("今天 {h:02}:{mi:02}", "Today {h:02}:{mi:02}")
    } else if (y, mo, d) == (yy, ymo, yd) {
        crate::localized_format!("昨天 {h:02}:{mi:02}", "Yesterday {h:02}:{mi:02}")
    } else if y == ny {
        crate::localized_format!("{mo}月{d}日", "{mo:02}-{d:02}")
    } else {
        if crate::settings::english() { format!("{y}-{mo:02}-{d:02}") } else { format!("{y}年{mo}月{d}日") }
    }
}

/// Header status time ("刚刚", "5 分钟前", then like `format_time`).
pub fn format_ago(secs: u64) -> String {
    let ago = crate::time::now().saturating_sub(secs);
    if ago < 60 {
        crate::settings::text("刚刚").into()
    } else if ago < 3600 {
        crate::localized_format!("{} 分钟前", "{} min ago", ago / 60)
    } else {
        format_time(secs)
    }
}

/// Full date ("2025年12月2日"), as in 文档信息.
pub fn format_date(secs: u64) -> String {
    let (y, mo, d, _, _) = civil(secs);
    if crate::settings::english() { format!("{y}-{mo:02}-{d:02}") } else { format!("{y}年{mo}月{d}日") }
}

/// Relative time for 文档信息 ("几秒前", "3 分钟前", "2 小时前", then the date).
pub fn format_relative(secs: u64) -> String {
    let ago = crate::time::now().saturating_sub(secs);
    match ago {
        0..60 => crate::settings::text("几秒前").into(),
        60..3600 => crate::localized_format!("{} 分钟前", "{} min ago", ago / 60),
        3600..86_400 => crate::localized_format!("{} 小时前", "{} hours ago", ago / 3600),
        _ => format_date(secs),
    }
}

/// Search result times (`space.date_format.*`): "刚刚", "N 分钟前",
/// "今天 / 昨天 / 前天 HH:mm", "M月D日 HH:mm", across years "YYYY年M月D日".
pub fn format_visit(secs: u64) -> String {
    let now = crate::time::now();
    let ago = now.saturating_sub(secs);
    if ago < 60 {
        return crate::settings::tr("刚刚", "just now").into();
    }
    if ago < 3600 {
        return if crate::settings::english() { format!("{} min ago", ago / 60) } else { format!("{} 分钟前", ago / 60) };
    }
    let (y, mo, d, h, mi) = civil(secs);
    let day = |back: u64| {
        let (y, mo, d, _, _) = civil(now.saturating_sub(back * 86_400));
        (y, mo, d)
    };
    let names = if crate::settings::english() { ["Today", "Yesterday", "Two days ago"] } else { ["今天", "昨天", "前天"] };
    if let Some(name) = (0..3).find(|&b| day(b) == (y, mo, d)).map(|b| names[b as usize]) {
        format!("{name} {h:02}:{mi:02}")
    } else if y == civil(now).0 {
        if crate::settings::english() { format!("{mo:02}-{d:02} {h:02}:{mi:02}") } else { format!("{mo}月{d}日 {h:02}:{mi:02}") }
    } else {
        if crate::settings::english() { format!("{y}-{mo:02}-{d:02}") } else { format!("{y}年{mo}月{d}日") }
    }
}
