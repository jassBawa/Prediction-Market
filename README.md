# Prediction Market Backend System

A high-performance, decentralized prediction market platform built with Rust, featuring real-time order matching, blockchain integration, and event-driven architecture.

## 🏗️ Architecture Overview

This project implements a microservices-based prediction market system with the following core components:

```
┌─────────────────┐
│   Backend API   │  ← REST API server (Axum)
│   (backend)     │     - Market management
│                 │     - Order placement & execution
│                 │     - Authentication (Privy)
└────────┬────────┘
         │
    ┌────┴────┬──────────────┬──────────────┐
    │         │              │              │
┌───▼───┐ ┌──▼──────┐  ┌────▼─────┐  ┌────▼────┐
│   DB  │ │ Event   │  │ Matching │  │  PM-CLI │
│       │ │Listener │  │  Engine  │  │         │
└───────┘ └─────────┘  └──────────┘  └─────────┘
    │         │              │
    │         │              │
    └─────────┴──────────────┘
         │
    ┌────▼─────┐
    │ Solana   │
    │ Blockchain│
    └──────────┘
```

### Component Responsibilities

#### **Backend** (`backend/`)
The main REST API server that handles:
- **Market Operations**: Listing, fetching, and querying prediction markets
- **Order Management**: Placing, canceling, splitting, and merging orders
- **Orderbook**: Real-time orderbook snapshots for each market
- **Authentication**: Privy-based JWT authentication middleware
- **Blockchain Integration**: Solana client for on-chain transaction execution
- **Matching Engine Integration**: Communicates with the matching engine for order processing

**Key Technologies:**
- Axum web framework
- Privy for authentication
- Anchor client for Solana program interaction
- PostgreSQL via SQLx

#### **Database** (`db/`)
A shared database library providing:
- **Database Models**: Market entities and data structures
- **Query Functions**: Type-safe database operations using SQLx
- **Migrations**: Schema management with SQLx migrations
- **Connection Pooling**: Efficient database connection management

**Schema Highlights:**
- Markets table with on-chain address mapping
- Support for Yes/No binary markets
- Market resolution tracking
- Indexed queries for performance

#### **Event Listener** (`event-listener/`)
A real-time blockchain event monitoring service that:
- **Subscribes** to Solana program logs via WebSocket
- **Parses Events**: Decodes `MarketInitialized` and `MarketSettled` events
- **Syncs State**: Automatically creates markets in the database when initialized on-chain
- **Updates Resolution**: Marks markets as resolved when settled on-chain

**Event Types:**
- `MarketInitialized`: New market creation events
- `MarketSettled`: Market resolution events

#### **Matching Engine** (`matching-engine/`)
An in-memory order matching engine that:
- **Orderbook Management**: Maintains separate orderbooks for Yes/No shares
- **Order Matching**: Implements price-time priority matching algorithm
- **Trade Execution**: Generates trade fills for matched orders
- **Snapshot API**: Provides real-time market snapshots

**Features:**
- Dual orderbook system (Yes/No shares)
- Bid/Ask order matching
- Partial fill support
- Real-time orderbook snapshots

#### **PM-CLI** (`pm-cli/`)
A command-line interface for market administration:
- **Create Markets**: Initialize new prediction markets on-chain
- **Resolve Markets**: Settle markets with outcomes
- **List Markets**: Query on-chain market data

**Use Cases:**
- Market creation by administrators
- Market resolution after events occur
- On-chain data inspection

## 🚀 Getting Started

### Prerequisites

- **Rust** (latest stable version)
- **PostgreSQL** 16+ (or use Docker Compose)
- **Solana CLI** (for keypair management)
- **Anchor** (for Solana program interaction)

### Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd Prediction-Market
   ```

2. **Start PostgreSQL database**
   ```bash
   docker-compose up -d
   ```
   This starts a PostgreSQL instance on `localhost:5432` with:
   - User: `admin`
   - Password: `password`
   - Database: `postgres_db`

3. **Configure environment variables**
   
   Copy the `.env.example` files to `.env` in each workspace member:
   ```bash
   cp backend/.env.example backend/.env
   cp event-listener/.env.example event-listener/.env
   ```
   
   See the `.env.example` files for required configuration.

4. **Run database migrations**
   ```bash
   cd db
   cargo run
   ```
   Migrations run automatically when the database pool is initialized.

5. **Start the services**

   **Backend API:**
   ```bash
   cd backend
   cargo run
   ```
   Server starts on `http://0.0.0.0:8080` (configurable via `SERVER_ADDR`)

   **Event Listener:**
   ```bash
   cd event-listener
   cargo run
   ```
   Runs continuously, listening for on-chain events.

## 📡 API Endpoints

### Public Endpoints

- `GET /markets` - List all markets
- `GET /markets/{address}` - Get market details by on-chain address

### Protected Endpoints (Require Authentication)

- `POST /orders/open` - Place a new order
- `POST /orders/close` - Cancel an existing order
- `GET /orderbook/{market_id}` - Get orderbook snapshot
- `POST /orders/split` - Split an order into smaller orders
- `POST /orders/merge` - Merge multiple orders

**Authentication:** Include `privy-id-token` header with Privy JWT token.

## 🔄 Data Flow

### Market Creation Flow

1. **Admin creates market** via `pm-cli`:
   ```bash
   cargo run --bin pm-cli -- create-market \
     --market-id 1 \
     --metadata "Will Bitcoin reach $100k by 2025?" \
     --end-time 1735689600
   ```

2. **On-chain event emitted** → `MarketInitialized` event

3. **Event listener detects** event via WebSocket subscription

4. **Database updated** → Market record created with on-chain address

5. **API serves market** → Available via `GET /markets`

### Order Placement Flow

1. **User places order** → `POST /orders/open` with order details

2. **Backend validates** → Authentication, market existence, order parameters

3. **Matching engine processes** → Order matched against existing orders

4. **Trades generated** → Partial or full fills returned

5. **On-chain execution** → Matched orders executed via Solana transactions

6. **Orderbook updated** → Real-time snapshot available

### Market Resolution Flow

1. **Admin resolves market** via `pm-cli`:
   ```bash
   cargo run --bin pm-cli -- resolve-market \
     --market-id 1 \
     --outcome "Yes"
   ```

2. **On-chain event emitted** → `MarketSettled` event

3. **Event listener updates** → Market marked as resolved in database

4. **API reflects status** → Market shows resolved state

## 🏛️ Architecture Patterns

### Microservices Architecture
Each component is a separate Rust crate with clear boundaries:
- **Loose coupling** via shared database and message passing
- **Independent deployment** of services
- **Scalability** through horizontal scaling

### Event-Driven Design
- **Blockchain events** trigger database updates
- **Order matching** uses async message passing
- **Real-time updates** via WebSocket subscriptions

### Type Safety
- **SQLx compile-time queries** prevent SQL injection
- **Strong typing** throughout the Rust codebase
- **Anchor types** for Solana program interaction

### Database-First Approach
- **Migrations** version control schema changes
- **Type-safe queries** with SQLx
- **Connection pooling** for performance

## 🧪 Development

### Workspace Structure

This is a Cargo workspace with shared dependencies:

```toml
[workspace]
members = [
    "db",
    "backend",
    "event-listener",
    "matching-engine",
    "pm-cli",
]
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Test specific component
cargo test -p backend
cargo test -p matching-engine
```

### Database Migrations

Migrations are in `db/migrations/` and run automatically on pool initialization.

To create a new migration:
```bash
cd db
sqlx migrate add <migration_name>
```

## 🔐 Security Considerations

- **Authentication**: Privy JWT tokens validated on protected routes
- **Keypair Management**: Admin keypairs stored securely, never committed
- **SQL Injection**: Prevented via SQLx compile-time query checking
- **CORS**: Configured for specific origins
- **Environment Variables**: Sensitive data in `.env` files (gitignored)

## 📦 Dependencies

### Core Technologies
- **Axum**: Modern async web framework
- **SQLx**: Type-safe SQL with compile-time verification
- **Anchor**: Solana program framework
- **Tokio**: Async runtime
- **Privy**: Authentication service

### Blockchain
- **Solana SDK**: Blockchain interaction
- **SPL Token**: Token program integration
- **Anchor Client**: Program client library

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

**Built with Rust 🦀 for performance, reliability, and type safety.**
