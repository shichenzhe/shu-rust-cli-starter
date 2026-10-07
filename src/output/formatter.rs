use anyhow::Result;
use serde::Serialize;

/// 通用表格输出：按显示宽度对齐（CJK 双宽），空数据显示提示
pub fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "没有数据".to_string();
    }

    // 每列宽度 = 表头与所有单元格的最大显示宽度
    let mut widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }

    let mut output = String::new();
    let header_row: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
    output.push_str(&format_aligned_row(&header_row, &widths));
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    output.push_str(&format_aligned_row(&sep, &widths));
    for row in rows {
        output.push_str(&format_aligned_row(row, &widths));
    }
    output
}

/// CSV 输出（表头 + 数据行）
pub fn format_csv(headers: &[&str], rows: &[Vec<String>]) -> Result<String> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(headers)?;
    for row in rows {
        wtr.write_record(row)?;
    }
    let bytes = wtr.into_inner()?;
    Ok(String::from_utf8(bytes)?)
}

/// JSON 输出（pretty）
pub fn format_json<T: Serialize>(items: &[T]) -> Result<String> {
    Ok(serde_json::to_string_pretty(items)?)
}

/// 单行按列宽对齐（最后一列不补空格）
fn format_aligned_row(cells: &[String], widths: &[usize]) -> String {
    let mut line = String::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            line.push_str("  ");
        }
        line.push_str(cell);
        if i < cells.len() - 1 {
            let pad = widths[i].saturating_sub(display_width(cell));
            line.push_str(&" ".repeat(pad));
        }
    }
    line.push('\n');
    line
}

/// 估算字符串在等宽终端的显示宽度（CJK 及全角字符计 2 列）
fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let cp = u32::from(c);
            let wide = matches!(
                cp,
                0x1100..=0x115F
                    | 0x2E80..=0xA4CF
                    | 0xAC00..=0xD7A3
                    | 0xF900..=0xFAFF
                    | 0xFE30..=0xFE4F
                    | 0xFF00..=0xFF60
                    | 0xFFE0..=0xFFE6
            );
            if wide {
                2
            } else {
                1
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_aligns_cjk_as_double_width() {
        let out = format_table(
            &["名称", "星数"],
            &[
                vec!["rust".to_string(), "1".to_string()],
                vec!["中文项目名".to_string(), "12345".to_string()],
            ],
        );
        let lines: Vec<&str> = out.lines().collect();
        // 列宽由 CJK 双宽的 "中文项目名"（显示宽 10）决定：
        // "名称"/"rust" 显示宽 4，各补 6 空格 + 2 分隔空格后接第二列
        assert!(lines[0].starts_with("名称        星数"));
        assert!(lines[2].starts_with("rust        1"));
        // 满宽 CJK 行补 0 空格，仅剩 2 个列分隔空格
        assert!(lines[3].starts_with("中文项目名  12345"));
        // 分隔线按列宽（10 / 5）绘制，锁定第 0 列确为 CJK 双宽决定
        assert_eq!(lines[1], "----------  -----");
    }

    #[test]
    fn table_empty_message() {
        assert_eq!(format_table(&["a"], &[]), "没有数据");
    }

    #[test]
    fn csv_quotes_commas() {
        let out = format_csv(
            &["name", "desc"],
            &[vec!["a,b".to_string(), "ok".to_string()]],
        )
        .unwrap();
        assert!(out.contains("\"a,b\""));
    }

    #[test]
    fn json_pretty_output() {
        #[derive(Serialize)]
        struct Item {
            n: u32,
        }
        let out = format_json(&[Item { n: 1 }]).unwrap();
        assert!(out.contains("\"n\": 1"));
    }
}
