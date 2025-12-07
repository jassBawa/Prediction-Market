-- Add migration script here
ALTER TABLE markets
ADD COLUMN collateral_mint TEXT,
ADD COLUMN collateral_vault TEXT,
ADD COLUMN yes_mint TEXT,
ADD COLUMN no_mint TEXT,
ADD COLUMN bump SMALLINT;
