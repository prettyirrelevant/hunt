use std::collections::{BTreeSet, HashMap, HashSet};

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{PgExecutor, PgPool};

use super::model::{Job, Posting, Stage};

/// Inserts unseen postings and returns their ids. Seen ones refresh `last_seen`.
pub async fn add(db: &PgPool, postings: &[Posting]) -> Result<Vec<i64>> {
    let mut tx = db.begin().await?;
    // Two sweeps at once would each miss the other's twins and store a role twice.
    sqlx::query("select pg_advisory_xact_lock(hashtext('jobs.add'))").execute(&mut *tx).await?;
    let keys: Vec<&str> = postings.iter().map(|p| p.key.as_str()).collect();
    let seen: HashSet<String> =
        sqlx::query_scalar("update jobs set last_seen = now() where key = any($1) returning key")
            .bind(&keys)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    let mut fingerprints = HashSet::new();
    let unseen: Vec<(&Posting, String)> = postings
        .iter()
        .filter(|p| !seen.contains(&p.key))
        .map(|p| (p, fingerprint(&p.company, &p.title)))
        .filter(|(_, f)| fingerprints.insert(f.clone()))
        .collect();

    let (prints, posted): (Vec<&str>, Vec<Option<DateTime<Utc>>>) =
        unseen.iter().map(|(p, f)| (f.as_str(), p.posted_at)).unzip();
    let twins: HashSet<String> = sqlx::query_scalar(
        "update jobs j set last_seen = now(),
             flags = case when n.posted_at - j.posted_at >= interval '14 days' and not 'reposted' = any(j.flags)
                          then array_append(j.flags, 'reposted') else j.flags end
         from unnest($1::text[], $2::timestamptz[]) as n (fingerprint, posted_at)
         where j.fingerprint = n.fingerprint
         returning j.fingerprint",
    )
    .bind(&prints)
    .bind(&posted)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    let rows: Vec<Value> = unseen
        .iter()
        .filter(|(_, f)| !twins.contains(f))
        .map(|(p, f)| {
            json!({
                "key": p.key, "fingerprint": f, "source": p.source, "url": p.url, "apply_url": p.apply_url,
                "company": p.company, "title": p.title, "location": p.location, "work_mode": p.work_mode.as_str(),
                "regions": p.regions, "countries": p.countries, "visa": p.visa, "relocation": p.relocation,
                "salary_min": p.salary_min, "salary_max": p.salary_max, "currency": p.currency,
                "seniority": p.seniority, "skills": p.skills, "requirements": p.requirements,
                "description": p.description, "posted_at": p.posted_at,
            })
        })
        .collect();
    let ids = sqlx::query_scalar(
        "insert into jobs (key, fingerprint, source, url, apply_url, company, title, location, work_mode,
            regions, countries, visa, relocation, salary_min, salary_max, currency, seniority, skills,
            requirements, description, posted_at)
         select key, fingerprint, source, url, apply_url, company, title, location, work_mode,
            regions, countries, visa, relocation, salary_min, salary_max, currency, seniority, skills,
            requirements, description, posted_at
         from jsonb_populate_recordset(null::jobs, $1)
         on conflict (key) do nothing
         returning id",
    )
    .bind(Value::Array(rows))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ids)
}

/// Company plus the set of title words, so word order between boards does not matter.
fn fingerprint(company: &str, title: &str) -> String {
    const COMPANY_NOISE: [&str; 9] = ["inc", "ltd", "llc", "gmbh", "corp", "co", "plc", "bv", "sa"];
    const TITLE_NOISE: [&str; 8] = ["remote", "m", "w", "d", "f", "x", "h", "all"];
    let words = |s: &str| {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let company: String = words(company).into_iter().filter(|w| !COMPANY_NOISE.contains(&w.as_str())).collect();
    let title: BTreeSet<String> = words(title).into_iter().filter(|w| !TITLE_NOISE.contains(&w.as_str())).collect();
    format!("{company}|{}", title.into_iter().collect::<Vec<_>>().join(" "))
}

pub async fn get(db: &PgPool, id: i64) -> Result<Job> {
    Ok(sqlx::query_as(crate::select_jobs!("where id = $1")).bind(id).fetch_one(db).await?)
}

pub async fn get_many(db: &PgPool, ids: &[i64]) -> Result<Vec<Job>> {
    Ok(sqlx::query_as(crate::select_jobs!("where id = any($1) order by id")).bind(ids).fetch_all(db).await?)
}

/// Moves each job to `to` and records why.
pub async fn move_to(db: &PgPool, moves: &[(i64, &str)], to: Stage) -> Result<()> {
    shift(db, moves, None, to).await.map(drop)
}

/// Moves only jobs still at `from`, and returns the ids that moved.
pub async fn move_from(db: &PgPool, from: Stage, moves: &[(i64, &str)], to: Stage) -> Result<Vec<i64>> {
    shift(db, moves, Some(from), to).await
}

async fn shift(db: &PgPool, moves: &[(i64, &str)], from: Option<Stage>, to: Stage) -> Result<Vec<i64>> {
    let ids: Vec<i64> = moves.iter().map(|&(id, _)| id).collect();
    let why: HashMap<i64, &str> = moves.iter().copied().collect();
    let mut tx = db.begin().await?;
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "select id, company, title, stage from jobs
         where id = any($1) and stage <> $2 and ($3::text is null or stage = $3)
         order by id for update",
    )
    .bind(&ids)
    .bind(to.as_str())
    .bind(from.map(Stage::as_str))
    .fetch_all(&mut *tx)
    .await?;

    let (mut changed, mut froms, mut whys, mut bodies) = (vec![], vec![], vec![], vec![]);
    for (id, company, title, from) in rows {
        let Some(stage) = Stage::parse(&from) else { bail!("job {id} has an unknown stage {from}") };
        let reason = why[&id];
        let mut body = format!("{company} · {title}: {} → {}", stage.label(), to.label());
        if !reason.is_empty() {
            body = format!("{body}. {reason}");
        }
        changed.push(id);
        froms.push(from);
        whys.push(reason);
        bodies.push(body);
    }
    sqlx::query("update jobs set stage = $2, stage_at = now() where id = any($1)")
        .bind(&changed)
        .bind(to.as_str())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "insert into events (job_id, kind, body, data)
         select id, 'stage', body, jsonb_build_object('from', from_stage, 'to', $5::text, 'why', why)
         from unnest($1::bigint[], $2::text[], $3::text[], $4::text[]) as m (id, body, from_stage, why)",
    )
    .bind(&changed)
    .bind(&bodies)
    .bind(&froms)
    .bind(&whys)
    .bind(to.as_str())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(changed)
}

pub async fn record(db: impl PgExecutor<'_>, job_id: Option<i64>, kind: &str, body: &str, data: Value) -> Result<()> {
    sqlx::query("insert into events (job_id, kind, body, data) values ($1, $2, $3, $4)")
        .bind(job_id)
        .bind(kind)
        .bind(body)
        .bind(data)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn add_flags(db: &PgPool, flags: &[(i64, &str)]) -> Result<()> {
    let (ids, names): (Vec<i64>, Vec<&str>) = flags.iter().copied().unzip();
    sqlx::query(
        "update jobs j set flags = j.flags || array(select f from unnest(n.flags) f where not f = any(j.flags))
         from (select id, array_agg(distinct flag) as flags from unnest($1::bigint[], $2::text[]) as t (id, flag)
               group by id) n
         where j.id = n.id",
    )
    .bind(&ids)
    .bind(&names)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn recent_decisions(db: &PgPool, limit: i64) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar(
        "select body from events where kind = 'stage' and data ->> 'to' in ('approved', 'skipped')
         order by at desc limit $1",
    )
    .bind(limit)
    .fetch_all(db)
    .await?)
}
