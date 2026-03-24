# Stellar-Save Roadmap

This document outlines the planned development milestones for Stellar-Save. Each version builds on the previous, progressively expanding functionality, usability, and reach.

---

## v1.0 — XLM-Only Groups (Current)

**Status:** ✅ In Progress

The foundation of the protocol. This version establishes the core ROSCA mechanics on Stellar Soroban using native XLM only.

### Scope

- **Group Management**
  - Create groups with configurable contribution amount, cycle duration, and max members
  - Update group parameters while in `Pending` state
  - Delete groups before activation
  - List and paginate groups

- **Membership**
  - Join and leave groups
  - Assign payout positions (sequential or random)
  - Track member profiles and contribution history

- **Contributions**
  - Contribute fixed XLM amount per cycle
  - Track contribution status per member per cycle
  - Enforce contribution deadlines
  - Detect missed contributions

- **Payouts**
  - Automatic payout execution when all members contribute
  - Rotating recipient selection based on payout position
  - Payout history and record tracking

- **Security & Admin**
  - Admin-controlled contract configuration
  - Emergency pause / unpause
  - Rate limiting for group creation and joining
  - Reentrancy protection
  - Signature verification utility (`verify_signature`)

- **Contract Infrastructure**
  - Comprehensive error types with categorization
  - Event emission for all state changes
  - Persistent storage with O(1) key lookups
  - Full unit test coverage

---

## v1.1 — Custom Token Support

**Status:** 🔜 Planned

Expand beyond XLM to support any Stellar token, enabling stablecoin-based savings groups.

### Scope

- **Token Integration**
  - Support for SEP-41 compliant Stellar tokens (USDC, EURC, etc.)
  - Token allowlist — admin-approved tokens only
  - Per-group token configuration set at creation time
  - Token balance validation before contribution acceptance

- **Multi-Token Groups**
  - Groups denominated in a single token (not mixed)
  - Token metadata display (symbol, decimals, issuer)
  - Contribution amounts expressed in token units

- **Frontend Updates**
  - Token selector when creating a group
  - Balance display in group's token denomination
  - Token approval flow for non-XLM contributions

---

## v2.0 — Flexible Payouts & Penalty Mechanisms

**Status:** 🔜 Planned

Introduce configurable payout schedules and financial accountability for missed contributions.

### Scope

- **Flexible Payout Schedules**
  - Configurable payout order: sequential, random, or bid-based
  - Early payout requests with group consensus
  - Partial cycle completion handling

- **Penalty Mechanisms**
  - Configurable late contribution penalty (percentage or flat fee)
  - Grace period before penalty is applied
  - Penalty pool distribution to other members
  - Member removal after repeated missed contributions

- **Dispute Resolution**
  - On-chain voting for edge case decisions
  - Majority-vote group actions (e.g. remove a member, pause a cycle)

- **Enhanced Cycle Management**
  - Cycle extension by group vote
  - Mid-cycle member replacement (with group approval)
  - Cycle history and audit trail

---

## v3.0 — Enhanced Frontend UI

**Status:** 🔜 Planned

A polished, production-ready web application with full wallet integration and real-time updates.

### Scope

- **Dashboard**
  - Personal savings overview (active groups, total contributed, upcoming payout)
  - Group activity feed with real-time updates
  - Contribution reminders and notifications

- **Group Management UI**
  - Full group creation wizard with validation
  - Member management interface
  - Payout schedule visualization (timeline view)
  - Cycle progress indicators

- **Wallet Integration**
  - Freighter wallet connect / disconnect
  - Multi-wallet support (Albedo, xBull)
  - Transaction signing flow with confirmation dialogs
  - Network switching (testnet / mainnet)

- **Analytics**
  - Personal savings history charts
  - Group health metrics (contribution rate, on-time %)
  - Payout projection calculator

- **Accessibility & UX**
  - Mobile-responsive design
  - Dark / light mode
  - Internationalization (i18n) support
  - ARIA-compliant components

---

## v4.0 — Mobile App & Fiat On/Off-Ramps

**Status:** 🔜 Planned

Bring Stellar-Save to mobile devices and bridge the gap between crypto and traditional finance.

### Scope

- **Mobile Application**
  - React Native app for iOS and Android
  - Biometric authentication (Face ID / fingerprint)
  - Push notifications for contribution deadlines and payouts
  - Offline-capable with sync on reconnect

- **Fiat On/Off-Ramps**
  - Integration with SEP-6 / SEP-24 anchors for fiat deposits and withdrawals
  - Local currency display alongside token amounts
  - Bank transfer and mobile money support (targeting African markets)

- **Community Features**
  - In-app group chat
  - Invite members via link or QR code
  - Group reputation scores based on contribution history

- **Expanded Reach**
  - USSD interface for feature phones (no smartphone required)
  - Agent network for cash-in / cash-out in underserved areas
  - Multi-language support (Yoruba, Igbo, Hausa, Swahili, French)

---

## Milestone Summary

| Version | Focus | Status |
|---------|-------|--------|
| v1.0 | XLM-only ROSCA core | ✅ In Progress |
| v1.1 | Custom token support | 🔜 Planned |
| v2.0 | Flexible payouts & penalties | 🔜 Planned |
| v3.0 | Enhanced frontend UI | 🔜 Planned |
| v4.0 | Mobile app & fiat on/off-ramps | 🔜 Planned |

---

## Contributing to the Roadmap

Have a feature idea or want to work on a planned milestone? 

- Open a [GitHub Issue](https://github.com/Xoulomon/Stellar-Save/issues) to propose or discuss features
- Check issues labeled `wave-ready` to find funded contribution opportunities
- See [CONTRIBUTING.md](../CONTRIBUTING.md) for how to get started

---

**Built with ❤️ for financial inclusion on Stellar**
