use anyhow::Result;
use sqlx::PgPool;

use crate::models::{Market, MarketRow};

pub async fn create_market(
    pool: &PgPool,
    market_address: &str,
    creator_address: &str,
    program_id: &str,
    title: &str,
    description: Option<&str>,
    category: Option<&str>,
    yes_option: &str,
    no_option: &str,
    end_timestamp: i64,
    created_slot: i64,
) -> Result<i64> {
    let row = sqlx::query!(
        r#"
        INSERT INTO markets (
            market_address,
            creator_address,
            program_id,
            title,
            description,
            category,
            yes_option,
            no_option,
            end_timestamp,
            created_slot
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
        RETURNING id
        "#,
        market_address,
        creator_address,
        program_id,
        title,
        description,
        category,
        yes_option,
        no_option,
        end_timestamp,
        created_slot
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

pub async fn get_market_by_address(pool: &PgPool, address: &str) -> Result<Option<Market>> {
    let row = sqlx::query_as::<_, MarketRow>(
        r#"
        SELECT
            id,
            market_address,
            creator_address,
            program_id,
            title,
            description,
            category,
            yes_option,
            no_option,
            end_timestamp,
            created_slot,
            resolved,
            resolved_outcome,
            created_at,
            updated_at
        FROM markets
        WHERE market_address = $1
        LIMIT 1
        "#,
    )
    .bind(address)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

pub async fn get_market_by_id(pool: &PgPool, id: i64) -> Result<Option<Market>> {
    let row = sqlx::query_as::<_, MarketRow>(
        r#"
        SELECT * FROM markets
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

pub async fn list_markets(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Market>> {
    let rows = sqlx::query_as::<_, MarketRow>(
        r#"
        SELECT * FROM markets
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

pub async fn list_resolved_markets(pool: &PgPool) -> Result<Vec<Market>> {
    let rows = sqlx::query_as::<_, MarketRow>(
        r#"
        SELECT * FROM markets
        WHERE resolved = TRUE
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

pub async fn list_active_markets(pool: &PgPool) -> Result<Vec<Market>> {
    let rows = sqlx::query_as::<_, MarketRow>(
        r#"
        SELECT *
        FROM markets
        WHERE resolved = FALSE
        ORDER by end_timestamp ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

pub async fn update_market_resolution(
    pool: &PgPool,
    market_address: &str,
    resolved_outcome: bool,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE markets
        SET resolved = TRUE,
            resolved_outcome = $2,
            updated_at = NOW()
        WHERE market_address = $1
        "#,
        market_address,
        resolved_outcome
    )
    .execute(pool)
    .await?;

    Ok(())
}
