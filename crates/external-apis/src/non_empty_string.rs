// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Non-empty string validation utilities
//!
//! This module provides [`NonEmptyString`], a wrapper type that guarantees string validity
//! at compile time through Rust's type system. This is particularly useful for configuration
//! values, API parameters, and other contexts where empty strings would be invalid.
//!
//! # Design Philosophy
//!
//! Rather than using runtime validation that can be forgotten or bypassed, `NonEmptyString`
//! makes invalid states unrepresentable by construction. Once you have a `NonEmptyString`
//! instance, you can be certain it contains at least one non-whitespace character.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use external_apis::NonEmptyString;
//!
//! // Valid construction
//! let username = NonEmptyString::new("alice").expect("Valid username");
//! let api_key = NonEmptyString::new("sk-1234567890").expect("Valid API key");
//!
//! // Invalid construction fails at creation time
//! let empty = NonEmptyString::new("");
//! assert!(empty.is_err());
//!
//! let whitespace_only = NonEmptyString::new("   \t\n  ");
//! assert!(whitespace_only.is_err());
//! ```
//!
//! ## Configuration Structs
//!
//! ```rust
//! use external_apis::NonEmptyString;
//!
//! #[derive(Debug)]
//! pub struct DatabaseConfig {
//!     pub host: NonEmptyString,
//!     pub username: NonEmptyString,
//!     pub database: NonEmptyString,
//!     pub port: u16,
//! }
//!
//! impl DatabaseConfig {
//!     pub fn new(
//!         host: impl Into<String>,
//!         username: impl Into<String>,
//!         database: impl Into<String>,
//!         port: u16,
//!     ) -> Result<Self, String> {
//!         Ok(Self {
//!             host: NonEmptyString::new(host)?,
//!             username: NonEmptyString::new(username)?,
//!             database: NonEmptyString::new(database)?,
//!             port,
//!         })
//!     }
//! }
//!
//! // Now DatabaseConfig is always valid by construction
//! let config = DatabaseConfig::new("localhost", "admin", "myapp", 5432).unwrap();
//! // No need for runtime validation - the type system guarantees validity
//! let connection_string = format!("{}@{}/{}",
//!     config.username.as_str(),
//!     config.host.as_str(),
//!     config.database.as_str()
//! );
//! ```
//!
//! ## String Parsing and Conversion
//!
//! ```rust
//! use external_apis::NonEmptyString;
//! use std::str::FromStr;
//!
//! // FromStr implementation
//! let parsed: NonEmptyString = "hello".parse().expect("Valid string");
//!
//! // Display formatting
//! let name = NonEmptyString::new("world").unwrap();
//! println!("Hello, {}", name); // Prints: Hello, world
//!
//! // AsRef<str> for generic string operations
//! fn process_string(s: &str) { /* ... */ }
//! process_string(name.as_ref());
//! ```
//!
//! # Memory Efficiency
//!
//! `NonEmptyString` uses `Box<str>` internally for memory efficiency:
//! - Minimal memory overhead compared to raw `String`
//! - Immutable after construction (no capacity/length separation)
//! - Efficient cloning for small to medium strings
//!
//! # Error Handling
//!
//! Validation errors return descriptive `String` messages that can be:
//! - Displayed to users for configuration errors
//! - Logged for debugging purposes
//! - Converted to custom error types in applications
//!
//! # Thread Safety
//!
//! `NonEmptyString` is `Send + Sync` and can be safely shared between threads
//! or stored in static contexts after construction.

use core::fmt;
use std::str::FromStr;

/// A non-empty string wrapper that ensures validity at construction
///
/// This type guarantees that the contained string:
/// - Is not empty (length > 0)
/// - Contains at least one non-whitespace character
/// - Is immutable after construction
///
/// # Memory Layout
///
/// Uses `Box<str>` internally for memory efficiency and immutability.
///
/// # Examples
///
/// ```rust
/// use external_apis::NonEmptyString;
///
/// let valid = NonEmptyString::new("hello").unwrap();
/// assert_eq!(valid.as_str(), "hello");
///
/// let invalid = NonEmptyString::new("   ");
/// assert!(invalid.is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(Box<str>);

impl NonEmptyString {
    /// Create a new `NonEmptyString` from any string-like input
    ///
    /// # Arguments
    ///
    /// * `s` - Any value that can be converted into a `String`
    ///
    /// # Returns
    ///
    /// * `Ok(NonEmptyString)` if the string contains at least one non-whitespace character
    /// * `Err(String)` with a descriptive error message if the string is empty or whitespace-only
    ///
    /// # Validation Rules
    ///
    /// - Empty strings (`""`) are rejected
    /// - Whitespace-only strings (`"   "`, `"\t\n"`) are rejected
    /// - Strings with leading/trailing whitespace are accepted (`" hello "` → valid)
    /// - Single non-whitespace characters are accepted (`"a"` → valid)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use external_apis::NonEmptyString;
    ///
    /// // Valid strings
    /// assert!(NonEmptyString::new("hello").is_ok());
    /// assert!(NonEmptyString::new(" world ").is_ok());
    /// assert!(NonEmptyString::new("a").is_ok());
    /// assert!(NonEmptyString::new("123").is_ok());
    ///
    /// // Invalid strings
    /// assert!(NonEmptyString::new("").is_err());
    /// assert!(NonEmptyString::new("   ").is_err());
    /// assert!(NonEmptyString::new("\t\n").is_err());
    /// ```
    ///
    /// # Performance
    ///
    /// This method performs one allocation to convert the input to `Box<str>`.
    /// The validation is O(n) where n is the string length.
    pub fn new(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();
        if s.trim().is_empty() {
            Err("String cannot be empty or whitespace-only".to_string())
        } else {
            Ok(NonEmptyString(s.into_boxed_str()))
        }
    }

    /// Get a string slice of the contained value
    ///
    /// # Returns
    ///
    /// A `&str` pointing to the validated string content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use external_apis::NonEmptyString;
    ///
    /// let username = NonEmptyString::new("alice").unwrap();
    /// assert_eq!(username.as_str(), "alice");
    ///
    /// // Can be used in string formatting
    /// println!("User: {}", username.as_str());
    /// ```
    ///
    /// # Performance
    ///
    /// This is a zero-cost operation that returns a direct reference to the
    /// internal `Box<str>` content.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NonEmptyString {
    /// Formats the `NonEmptyString` for display
    ///
    /// This allows `NonEmptyString` to be used directly in format strings
    /// and print statements without explicitly calling `.as_str()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use external_apis::NonEmptyString;
    ///
    /// let name = NonEmptyString::new("Alice").unwrap();
    /// println!("Hello, {}!", name); // Prints: Hello, Alice!
    ///
    /// let greeting = format!("Welcome, {}", name);
    /// assert_eq!(greeting, "Welcome, Alice");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for NonEmptyString {
    type Err = String;

    /// Parse a `NonEmptyString` from a string slice
    ///
    /// This enables using the `.parse()` method and automatic parsing
    /// in contexts that expect `FromStr` implementations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use external_apis::NonEmptyString;
    /// use std::str::FromStr;
    ///
    /// // Using parse() method
    /// let parsed: NonEmptyString = "hello".parse().expect("Valid string");
    /// assert_eq!(parsed.as_str(), "hello");
    ///
    /// // Using FromStr::from_str directly
    /// let parsed = NonEmptyString::from_str("world").unwrap();
    /// assert_eq!(parsed.as_str(), "world");
    ///
    /// // Parsing invalid strings returns error
    /// let invalid: Result<NonEmptyString, _> = "".parse();
    /// assert!(invalid.is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for NonEmptyString {
    /// Get a string slice reference
    ///
    /// This trait implementation allows `NonEmptyString` to be used with
    /// generic functions that accept `AsRef<str>`, providing seamless
    /// interoperability with the standard library and third-party code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use external_apis::NonEmptyString;
    ///
    /// let content = NonEmptyString::new("Hello, world!").unwrap();
    ///
    /// // Works with generic functions expecting AsRef<str>
    /// fn print_string<S: AsRef<str>>(s: S) {
    ///     println!("{}", s.as_ref());
    /// }
    /// print_string(&content);
    ///
    /// // Can be converted to &str seamlessly
    /// let s: &str = content.as_ref();
    /// assert_eq!(s, "Hello, world!");
    /// ```
    fn as_ref(&self) -> &str {
        &self.0
    }
}
