use std::path::Path;
use std::process::Command;

use super::{hex, home};
use crate::theme::Theme;

/*
 * Exports the dynamic shell theme directly into Flectar Mail's SQLite database
 * (~/.local/share/flectar-mail/flectar-mail.db) so Flectar Mail's colors match the desktop wallpaper.
 */
pub fn export(theme: &Theme) {
    let db_path = format!("{}/.local/share/flectar-mail/flectar-mail.db", home());
    if !Path::new(&db_path).exists() {
        return;
    }

    let dark_primary = hex(theme.accent);
    let dark_bg = hex(theme.background);
    let dark_surface = hex(theme.surface);
    let dark_text = hex(theme.text);
    let dark_border = hex(theme.border);

    let light_primary = hex(theme.accent);
    let light_bg = "#f2f2f0";
    let light_surface = "#ffffff";
    let light_text = "#202120";
    let light_border = "#d9d9d6";

    let py_code = r#"
import sqlite3, json, sys

db_path, dp, dbg, ds, dt, db, lp, lbg, ls, lt, lb = sys.argv[1:12]

try:
    con = sqlite3.connect(db_path)
    cur = con.cursor()
    row = cur.execute("SELECT value FROM app_settings WHERE key='settings';").fetchone()
    if not row:
        con.close()
        sys.exit(0)

    s = json.loads(row[0])
    s['themePreset'] = 'custom'
    s['theme'] = 'system'
    ct = s.get('customTheme', {})
    ct['darkPrimary'] = dp
    ct['darkPageBackground'] = dbg
    ct['darkSurface'] = ds
    ct['darkText'] = dt
    ct['darkBorder'] = db
    ct['lightPrimary'] = lp
    ct['lightPageBackground'] = lbg
    ct['lightSurface'] = ls
    ct['lightText'] = lt
    ct['lightBorder'] = lb
    s['customTheme'] = ct

    cur.execute("UPDATE app_settings SET value=? WHERE key='settings';", (json.dumps(s),))
    con.commit()
    con.close()
except Exception:
    pass
"#;

    let _ = Command::new("python3")
        .args([
            "-c",
            py_code,
            &db_path,
            &dark_primary,
            &dark_bg,
            &dark_surface,
            &dark_text,
            &dark_border,
            &light_primary,
            light_bg,
            light_surface,
            light_text,
            light_border,
        ])
        .status();
}
