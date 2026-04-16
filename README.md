<div align="center">
  <img src="src/assets/logo.png" alt="FilterEmail Logo" width="120" />
  <h1>FilterEmail</h1>
  <p><strong>High-performance desktop app for filtering and validating massive email lists.</strong></p>
  <p>Built with Tauri v2, Rust, React, and Tailwind CSS.</p>
</div>

<hr/>

## 🚀 Overview

FilterEmail is a blazing-fast desktop application designed to process enormous email lists (TXT/CSV) without consuming excessive RAM. It streams files line-by-line using a powerful Rust backend and provides a modern, responsive UI built with React and Tailwind CSS.

The application offers two main modes:
1.  **Basic Filter**: Instantly classify emails into categories (Invalid, Public, Edu/Gov, Targeted, Other, Duplicates).
2.  **DNS & SMTP Verify (Deep Scan)**: Validate domains in real-time by checking MX records, A record fallbacks, identifying parked/disposable domains, catching typos, and optionally pinging a VPS SMTP API for true deliverability checks.

## ✨ Key Features

### ⚡ Rust-Powered Core
-   **Stream Processing:** Reads and writes gigabyte-sized files chunk-by-chunk. Memory stays flat regardless of file size.
-   **High Concurrency:** Perform asynchronous DNS lookups (up to 50 concurrent requests) tailored for blazing speeds.
-   **Persistent DNS Cache:** SQLite-backed cache (TTL 6 hours) significantly dramatically speeds up repeated scans of the same domains.
-   **Tauri v2:** Ultra-lightweight desktop binary compared to traditional Electron apps.

### 🎨 Modern Dashboard UI
-   **Intuitive UX:** Drag-and-drop file support.
-   **Real-time Feedback:** Live progress bars, current scanning domain, and cache hit metrics.
-   **Beautiful Data Visualization:** Tailwind CSS-powered stat cards with smooth micro-animations.
-   **Bilingual:** Fully supports English and Vietnamese (`i18n`).
-   **History Log:** Automatically saves past processing runs for easy retrieval.

### 🛡 Multi-Layer Verification
-   **Tier 1:** Syntax & Structural checks (Regex, Length limits).
-   **Tier 2:** Provider Classification (Gmail, Yahoo, Edu, Gov, Custom Targets).
-   **Tier 3:** Deep DNS Scan (`hickory-resolver`). Detects:
    -   Valid MX Records
    -   A-Record Fallbacks
    -   Parked Domains (via known parking nameservers)
    -   Disposable Emails (throw-away providers)
    -   Common Typos (e.g., `gamil.com` -> `gmail.com`)
-   **Tier 4:** SMTP Verification (Integration ready via VPS API).

## 🛠 Tech Stack

-   **Frontend:** React 19, TypeScript, Tailwind CSS, Lucide React icons.
-   **Backend:** Rust, Tauri v2.
-   **Key Rust Crates:** `tokio` (async runtime), `hickory-resolver` (DNS), `rusqlite` (Cache), `reqwest` (HTTP Client).
-   **Build Tool:** Vite, Cargo.

## 📦 Installation & Running Locally

### Prerequisites
Make sure you have installed:
-   [Node.js](https://nodejs.org/) (v18+)
-   [Rust](https://rustup.rs/) (1.70+)
-   Target OS dependencies for Tauri (see [Tauri Setup Guide](https://tauri.app/v1/guides/getting-started/prerequisites)).

### Getting Started

1.  **Clone the repository**
    ```bash
    git clone https://github.com/HulkBetii/FilterEmail.git
    cd FilterEmail
    ```

2.  **Install frontend dependencies**
    ```bash
    npm install
    ```

3.  **Run the development server**
    ```bash
    npm run tauri dev
    ```
    This will start the Vite frontend on port `1420` and launch the Tauri desktop window.

4.  **Build for production**
    ```bash
    npm run tauri build
    ```
    The compiled binary will be located in `src-tauri/target/release/`.

## ⚙️ Configuration (Verification Mode)

When using the "Verify DNS" tab, you can configure several parameters:
-   **DNS Timeout:** How long to wait for a DNS response (Default: `1500ms`).
-   **Max Concurrent Lookups:** Balances speed vs. network pressure (Default: `40`).
-   **Persistent DNS Cache:** Toggle to save lookups to SQLite.
-   **SMTP Verify (VPS):** Input your VPS API endpoint and API Key. The backend sends `MX-Valid` domains to this proxy for SMTP Handshake validation.

## 📁 Output Structure

The application automatically creates neatly categorized files in your selected output folder. Example output:
```text
01_filter__hop_le.txt
02_filter__public_mail.txt
03_filter__edu_gov.txt
...
10_dns_domain_chet__dead.txt
11_dns_mx_hop_le__has_mx.txt
15_dns_parked.txt
20_smtp_gui_duoc__deliverable.txt
```

## 📝 License
Proprietary / MIT (Please specify your license here).
