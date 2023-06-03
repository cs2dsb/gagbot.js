use std::fmt::Write;

use rusqlite::Connection;
use tracing::error;

pub fn get_table_size_in_bytes(db: &Connection) -> anyhow::Result<Vec<(String, u64, u64)>> {
    let mut stmt = db.prepare(
        "SELECT name, SUM(pgsize) FROM dbstat
    WHERE name NOT LIKE 'sqlite_%'
    GROUP BY name",
    )?;

    let mut sizes: Vec<_> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, 0)))?
        .collect::<Result<_, _>>()?;

    if sizes.len() == 0 {
        return Ok(sizes);
    }

    if sizes.len() > 0 {
        let mut stmt_str = String::new();
        for (i, (name, _, _)) in sizes.iter().enumerate() {
            if i > 0 {
                stmt_str.push_str("\n UNION \n");
            }
            write!(&mut stmt_str, "SELECT '{name}', COUNT(1) FROM {}", name)?;
        }

        let mut stmt = db.prepare(&stmt_str)?;

        let counts: Vec<(String, u64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;

        if sizes.len() != counts.len() {
            // I have no idea how this is possible but it happened during debugging so this
            // is here to try and catch it if it happens again
            error!(
                "Table counts result rows != table size result rows.\n\tsizes:{:?}\n\tcounts{:?}",
                sizes, counts
            );
        } else {
            for i in 0..sizes.len() {
                sizes[i].2 = counts[i].1;
            }
        }
    }

    Ok(sizes)
}
