-- MARKETS TABLE
CREATE TABLE markets (
    id BIGSERIAL PRIMARY KEY,

    market_address TEXT NOT NULL UNIQUE,
    market_id TEXT NOT NULL UNIQUE,
    creator_address TEXT NOT NULL,
    program_id TEXT NOT NULL,

    -- static data
    title TEXT NOT NULL,
    description TEXT,
    category TEXT,
    yes_option TEXT DEFAULT 'Yes',
    no_option TEXT DEFAULT 'No',

    -- market settings
    end_timestamp BIGINT NOT NULL,
    created_slot BIGINT NOT NULL,
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_outcome TEXT ,

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT check_resolved_outcome
          CHECK (resolved_outcome IS NULL OR resolved_outcome IN ('Yes', 'No'))
);

-- Useful indexes
CREATE INDEX idx_markets_market_address ON markets(market_address);
CREATE INDEX idx_markets_category ON markets(category);
CREATE INDEX idx_markets_end_timestamp ON markets(end_timestamp);
