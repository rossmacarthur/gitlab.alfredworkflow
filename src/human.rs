use std::time::Duration;

use dairy::Cow;

pub fn format_ago(d: Duration) -> Cow<'static, str> {
    match d.as_secs() {
        x if x <= 44 => "a few seconds ago".into(),
        x if x <= 89 => "a minute ago".into(),
        x if x <= 44 * 60 => {
            let m = x as f32 / 60.;
            format!("{:.0} minutes ago", m.ceil()).into()
        }
        x if x <= 89 * 60 => "an hour ago".into(),
        x if x <= 21 * 60 * 60 => {
            let h = x as f32 / 60. / 60.;
            format!("{:.0} hours ago", h.ceil()).into()
        }
        x if x <= 35 * 60 * 60 => "a day ago".into(),
        x if x <= 25 * 24 * 60 * 60 => {
            let d = x as f32 / 24. / 60. / 60.;
            format!("{:.0} days ago", d.ceil()).into()
        }
        x if x <= 45 * 24 * 60 * 60 => "a month ago".into(),
        x if x <= 10 * 30 * 24 * 60 * 60 => {
            let m = x as f32 / 30. / 24. / 60. / 60.;
            format!("{:.0} months ago", m).into()
        }
        x if x <= 17 * 30 * 24 * 60 * 60 => "a year ago".into(),
        _ => {
            let y = d.as_secs_f32() / 12. / 30. / 24. / 60. / 60.;
            format!("{:.0} years ago", y).into()
        }
    }
}
