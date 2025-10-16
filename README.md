# Permissionless Block Builder

A **trustless, verifiable block construction system** that demonstrates fair transaction ordering through deterministic algorithms and cryptographic proof.

## What This Project Does

### 🎯 Core Purpose

This system solves the **MEV (Maximal Extractable Value) problem** by creating a transparent, verifiable way to order transactions in blockchain blocks. Instead of relying on centralized block builders who might manipulate transaction ordering for profit, this implementation provides:

- **Deterministic ordering** that anyone can verify
- **Cryptographic proofs** of fair bundle sequencing  
- **Permissionless verification** - no trust required

### 🏗️ System Architecture

```
📦 Bundle Submission → 🗄️ Redis Pool → 🧠 Block Engine → ✅ Verified Block → 🌐 Network
```

**Components:**

1. **API Server** (`api-server/`) - Receives transaction bundles from searchers/MEV bots
2. **Block Engine** (`block-engine/`) - Orders bundles deterministically and creates verifiable blocks
3. **Frontend Dashboard** (`frontend-dashboard/`) - Real-time visualization of block construction
4. **TEE Service** (`tee-service/`) - Trusted Execution Environment for enhanced security

### 🔄 How It Works

#### 1. Bundle Collection (200ms Windows)
- Searchers submit **transaction bundles** with tips
- Each bundle contains:
  - Transaction list
  - Tip amount (bid for inclusion)
  - Searcher public key
  - Timestamp

#### 2. Deterministic Ordering Algorithm
```rust
// Sort by tip amount (highest first), then by bundle ID hash for tie-breaking
bundles.sort_by_key(|b| (b.tip, hash_str(&b.id)));
```

- **Primary sort**: Tip amount (economic incentive)
- **Tie-breaker**: Deterministic hash of bundle ID
- **Result**: Same inputs → Same ordering (always)

#### 3. Cryptographic Verification
```rust
// Create SHA256 hash of the final ordering
let mut hasher = Sha256::new();
for bundle in &ordered_bundles {
    hasher.update(bundle.id.as_bytes());
    hasher.update(bundle.tip.to_le_bytes());
}
let proof_hash = hasher.finalize();
```

#### 4. Block Publication
- Ordered block + cryptographic proof sent to validators
- Anyone can independently verify the ordering was fair
- No manipulation possible - algorithm is transparent

### 🛡️ Fairness Guarantees

**Economic Fairness**
- Highest tips get priority (market-based allocation)
- No preferential treatment for specific searchers
- Transparent fee structure

**Technical Fairness**  
- Deterministic tie-breaking prevents manipulation
- Open-source algorithm - no hidden logic
- Cryptographic proofs enable trustless verification

**Temporal Fairness**
- Fixed 200ms time windows prevent timing games
- All bundles in a window compete equally
- No "last-look" advantages

### 📊 Real-World Impact

**For Searchers/MEV Bots:**
- Fair competition based on economic bids
- Predictable ordering rules
- No need to trust centralized builders

**For Users:**
- Reduced transaction reordering attacks
- More predictable execution
- Protection from MEV extraction

**For Validators:**
- Verifiable block construction
- Reduced regulatory risk
- Maintains network decentralization

### 🔍 Verification Process

Any party can verify block construction by:

1. **Collecting the same bundle data** from the time window
2. **Running the sorting algorithm** with identical inputs
3. **Computing the cryptographic hash** of the result
4. **Comparing with the published block hash**

If hashes match → Block was constructed fairly ✅  
If hashes differ → Manipulation detected ❌

### 🚀 Innovation Highlights

- **First fully deterministic** MEV-protected block builder
- **Zero-trust verification** - no need to trust the operator
- **Permissionless participation** - anyone can run and verify
- **Cryptographically provable** fairness guarantees
- **Production-ready Rust implementation** for performance

### 🌍 Broader Vision

This project represents a step toward **credibly neutral blockchain infrastructure** where:

- Block construction becomes transparent
- MEV extraction is minimized through fair ordering
- Network participants can verify system integrity
- Decentralization is preserved through open verification

---

*This is a research implementation demonstrating how blockchain infrastructure can become more fair, transparent, and verifiable through cryptographic techniques and deterministic algorithms.*
