# My Budget Server

A personal expense tracking backend server built with Rust and Axum, designed to be deployed as a RESTful API for budget management applications.

## 🚀 Features

- **User Authentication**: Secure registration and login with session-based authentication
- **Personal Data Isolation**: Each user gets their own SQLite database for complete data privacy
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
└── src/
    ├── main.rs                  # Main application + routing
    ├── auth.rs                  # Authentication & session handling
    ├── records.rs               # Expense records API + prediction
    ├── categories.rs            # Category management API
    ├── database.rs              # Database connections & operations
    └── models.rs                # Data structures & models
```

**Built with ❤️ for personal budget management**
