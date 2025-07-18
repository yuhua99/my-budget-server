# My Budget Server

A personal expense tracking backend server built with Rust and Axum, designed to be deployed as a RESTful API for budget management applications.

## 🚀 Features

- **User Authentication**: Secure registration and login with session-based authentication
- **Personal Data Isolation**: Each user gets their own Turso database for complete data privacy
- **Expense Management**: Full CRUD operations for expense records with categorization
- **Smart Predictions**: Automatic expense name suggestions based on similar past records
- **Category Management**: Flexible expense categorization system
- **RESTful API**: Clean REST endpoints for easy frontend integration

## 🛠️ Tech Stack

- **Framework**: Rust + Axum
- **Database**: Turso (per-user isolation)
- **Authentication**: Session-based with tower-sessions
- **Password Security**: Argon2 hashing
- **Architecture**: RESTful API

## 📂 Project Structure

```
my-budget-server/
├── Cargo.toml
├── .env
├── users.db                     # Main user database
├── data/                        # Individual user databases
│   └── user_*.db
├── src/
│   ├── main.rs                  # Main application + routing
│   ├── auth.rs                  # Authentication & session handling
│   ├── records.rs               # Expense records API + prediction
│   ├── categories.rs            # Category management API
│   ├── database.rs              # Database connections & operations
│   ├── lib.rs                   # Library exports
│   └── models.rs                # Data structures & models
├── tests/
│   ├── common/                  # Shared test utilities
│   ├── records_test.rs          # Records integration tests
│   └── helper_functions_test.rs # Helper function tests
└── benches/
    └── records_bench.rs         # Performance benchmarks
```

The server will start on `http://localhost:3000` by default.

## 🔧 Configuration

Create a `.env` file in the project root:

```env
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
DATABASE_PATH=./data
SESSION_SECRET=use openssl rand -hex 64 to generate your secret
```

## 🧪 Testing & Benchmarks

### Testing
The project includes comprehensive integration tests covering the records API functionality:

- **Integration Tests**: Full API testing with time-range filtering, pagination, and ordering
- **Unit Tests**: Database operations and helper function testing  
- **Test Coverage**: CRUD operations, data integrity, and edge cases
- **Isolated Testing**: Each test uses temporary databases for complete isolation

**Run Tests:**
```bash
cargo test                    # Run all tests
cargo test records_test       # Run specific test file
cargo test helper_functions   # Run helper function tests
```

### Benchmarks
Performance benchmarks using Criterion.rs for statistical analysis:

- **Database Operations**: Records creation and retrieval performance
- **Statistical Analysis**: Mean, median, and standard deviation metrics
- **HTML Reports**: Visual performance reports generated automatically

**Run Benchmarks:**
```bash
cargo bench                   # Run all benchmarks
cargo bench records_bench     # Run records benchmarks
```

Benchmark reports are generated in `target/criterion/` with detailed HTML visualizations.

**Built with ❤️ for personal budget management**
