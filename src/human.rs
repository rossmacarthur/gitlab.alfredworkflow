use std::time::Duration;

pub fn format_ago(d: Duration) -> String {
    match d.as_secs() {
        x if x <= 44 => "a few seconds ago".to_string(),
        x if x <= 89 => "a minute ago".to_string(),
        x if x <= 44 * 60 => {
            let m = x as f32 / 60 as f32;
            format!("{:.0} minutes ago", m.ceil())
        }
        x if x <= 89 * 60 => "an hour ago".to_string(),
        x if x <= 21 * 60 * 60 => {
            let h = x as f32 / 60 as f32 / 60 as f32;
            format!("{:.0} hours ago", h.ceil())
        }
        x if x <= 35 * 60 * 60 => "a day ago".to_string(),
        x if x <= 25 * 24 * 60 * 60 => {
            let d = x as f32 / 24 as f32 / 60 as f32 / 60 as f32;
            format!("{:.0} days ago", d.ceil())
        }
        x if x <= 45 * 24 * 60 * 60 => "a month ago".to_string(),
        x if x <= 10 * 30 * 24 * 60 * 60 => {
            let m = x as f32 / 30 as f32 / 24 as f32 / 60 as f32 / 60 as f32;
            format!("{:.0} months ago", m)
        }
        x if x <= 17 * 30 * 24 * 60 * 60 => "a year ago".to_string(),
        _ => {
            let y = d.as_secs_f32() / 12 as f32 / 30 as f32 / 24 as f32 / 60 as f32 / 60 as f32;
            format!("{:.0} years ago", y)
        }
    }
}
