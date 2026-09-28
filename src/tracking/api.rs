use leptos::prelude::*;

use super::model::{Class, Reply};
use crate::ui::error::Error;

#[server]
pub async fn get_replies() -> Result<Vec<Reply>, Error> {
    use std::sync::Arc;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    Ok(sqlx::query_as(
        "select m.id, m.at, m.sender, m.subject, m.snippet, m.job_id, j.company, j.title, m.class, m.confidence, m.status
         from messages m left join jobs j on j.id = m.job_id
         where m.status in ('needs_you', 'applied')
         order by m.status = 'needs_you' desc, m.at desc limit 60",
    )
    .fetch_all(&app.db)
    .await?)
}

#[server]
pub async fn resolve(id: i64, class: Class) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;

    Ok(super::service::resolve(&expect_context::<Arc<App>>(), id, class).await?)
}
