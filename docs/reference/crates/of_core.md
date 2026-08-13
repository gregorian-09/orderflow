# `of_core` Reference

> Generated from `crates/of_core/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.5.0`<br>
**Description:** Core domain models and analytics primitives for the Orderflow engine<br>
**Source:** [`crates/of_core/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_core/src)<br>
**Generated Rustdoc:** [open `of_core` Rustdoc](https://docs.rs/of_core/0.5.0/of_core/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- `default`: empty feature
- `tickbar`: `dep:tickbar`

## Local Dependencies

- No local workspace dependencies.

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `struct` | `SymbolId` | Canonical market symbol identifier used across venues | [`crates/of_core/src/lib.rs:60`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L60) | `present` |
| `enum` | `Side` | Trade or book side | [`crates/of_core/src/lib.rs:69`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L69) | `present` |
| `enum` | `BookAction` | Book mutation kind | [`crates/of_core/src/lib.rs:78`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L78) | `present` |
| `struct` | `BookUpdate` | Level-2 order book update | [`crates/of_core/src/lib.rs:87`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L87) | `present` |
| `struct` | `BookLevel` | One normalized price level in a materialized book snapshot | [`crates/of_core/src/lib.rs:110`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L110) | `present` |
| `struct` | `BookSnapshot` | Materialized order-book snapshot for a symbol | [`crates/of_core/src/lib.rs:121`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L121) | `present` |
| `struct` | `TradePrint` | Last-trade print/tick | [`crates/of_core/src/lib.rs:138`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L138) | `present` |
| `struct` | `AnalyticsSnapshot` | Aggregated analytics for a symbol/session | [`crates/of_core/src/lib.rs:157`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L157) | `present` |
| `struct` | `DerivedAnalyticsSnapshot` | Additive derived analytics computed from the current session accumulator state | [`crates/of_core/src/lib.rs:178`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L178) | `present` |
| `struct` | `SessionCandleSnapshot` | Session candle-style summary derived from the current analytics session | [`crates/of_core/src/lib.rs:193`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L193) | `present` |
| `struct` | `IntervalCandleSnapshot` | Rolling interval candle-style summary derived from recent session trades | [`crates/of_core/src/lib.rs:212`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L212) | `present` |
| `struct` | `CompletedBar` | A completed fixed-interval OHLCV bar | [`crates/of_core/src/lib.rs:237`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L237) | `present` |
| `struct` | `BookAnalyticsSnapshot` | Snapshot of book-derived analytics computed from an order book snapshot | [`crates/of_core/src/lib.rs:258`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L258) | `present` |
| `fn` | `compute_book_analytics` | Computes book-derived analytics from a materialized order book snapshot | [`crates/of_core/src/lib.rs:283`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L283) | `present` |
| `fn` | `compute_weighted_average_price` | Computes the weighted average price for an order of `qty` shares walking the book | [`crates/of_core/src/lib.rs:347`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L347) | `present` |
| `fn` | `compute_depth_slope` | Computes the depth slope — average volume decay per level away from the top of book | [`crates/of_core/src/lib.rs:393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L393) | `present` |
| `fn` | `compute_mid_price` | Returns the mid price from a book snapshot, or `None` if either side is empty | [`crates/of_core/src/lib.rs:424`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L424) | `present` |
| `fn` | `compute_effective_spread_bps` | Computes effective spread in basis points for a single trade | [`crates/of_core/src/lib.rs:434`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L434) | `present` |
| `fn` | `compute_realised_spread_bps` | Computes realised spread in basis points | [`crates/of_core/src/lib.rs:446`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L446) | `present` |
| `struct` | `SpreadTracker` | Tracks effective and realised spread for recent trades | [`crates/of_core/src/lib.rs:456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L456) | `present` |
| `struct` | `SpreadSample` | One recorded trade for spread tracking | [`crates/of_core/src/lib.rs:465`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L465) | `present` |
| `fn` | `new` | Creates a new tracker that retains up to `max_samples` recent trades | [`crates/of_core/src/lib.rs:476`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L476) | `present` |
| `fn` | `on_trade` | Records a trade with the prevailing mid price | [`crates/of_core/src/lib.rs:484`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L484) | `present` |
| `fn` | `last_effective_spread_bps` | Returns the effective spread in bps for the most recent trade | [`crates/of_core/src/lib.rs:498`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L498) | `present` |
| `fn` | `average_half_spread_cost_bps` | Returns the average half-spread cost (`effective_spread / 2`) over the last `window` trades | [`crates/of_core/src/lib.rs:506`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L506) | `present` |
| `fn` | `realised_spread_bps` | Returns the realised spread in bps for the trade `hold_ticks` ago | [`crates/of_core/src/lib.rs:523`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L523) | `present` |
| `fn` | `sample_count` | Returns the number of samples currently tracked | [`crates/of_core/src/lib.rs:536`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L536) | `present` |
| `fn` | `reset` | Clears all samples | [`crates/of_core/src/lib.rs:541`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L541) | `present` |
| `struct` | `BookEventTracker` | Tracks order-book update events for rate and size-distribution analytics | [`crates/of_core/src/lib.rs:548`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L548) | `present` |
| `struct` | `BookEventSample` | A single book update event for analytics | [`crates/of_core/src/lib.rs:557`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L557) | `present` |
| `fn` | `new` | Creates a new tracker retaining up to `max_events` recent events | [`crates/of_core/src/lib.rs:570`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L570) | `present` |
| `fn` | `on_book_update` | Records a book update event | [`crates/of_core/src/lib.rs:578`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L578) | `present` |
| `fn` | `event_count_in_window` | Returns the number of events in the time window `window_ns` (nanoseconds) per side | [`crates/of_core/src/lib.rs:598`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L598) | `present` |
| `fn` | `arrival_rate_per_sec` | Returns the per-side arrival (upsert) rate per second over `window_ns` | [`crates/of_core/src/lib.rs:622`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L622) | `present` |
| `fn` | `cancel_rate_per_sec` | Returns the per-side cancel (delete) rate per second over `window_ns` | [`crates/of_core/src/lib.rs:648`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L648) | `present` |
| `fn` | `event_volume_in_window` | Returns the total volume of order-book events per side in `window_ns` | [`crates/of_core/src/lib.rs:674`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L674) | `present` |
| `fn` | `event_count` | Returns the number of events recorded | [`crates/of_core/src/lib.rs:694`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L694) | `present` |
| `fn` | `reset` | Clears all events | [`crates/of_core/src/lib.rs:699`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L699) | `present` |
| `struct` | `BookEventAnalyticsSnapshot` | A snapshot of book-event analytics | [`crates/of_core/src/lib.rs:707`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L707) | `present` |
| `fn` | `is_empty` | Returns true if all fields are zero (no data) | [`crates/of_core/src/lib.rs:726`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L726) | `present` |
| `struct` | `ResiliencyTracker` | Tracks book depth before and after trades for resiliency metrics | [`crates/of_core/src/lib.rs:753`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L753) | `present` |
| `struct` | `ResiliencySample` | Book depth around a single trade | [`crates/of_core/src/lib.rs:762`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L762) | `present` |
| `fn` | `new` | Creates a new tracker with a maximum sample count | [`crates/of_core/src/lib.rs:779`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L779) | `present` |
| `fn` | `on_trade_pre` | Records book depth before a trade is applied | [`crates/of_core/src/lib.rs:788`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L788) | `present` |
| `fn` | `on_trade_post` | Records book depth after a trade and sets the post-trade depth | [`crates/of_core/src/lib.rs:803`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L803) | `present` |
| `fn` | `latest_recovery_time_ms` | Returns estimated recovery time in milliseconds for the most recent trade | [`crates/of_core/src/lib.rs:821`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L821) | `present` |
| `fn` | `latest_depth_elasticity` | Returns depth elasticity: `pre_trade_depth / recovery_time_ms` | [`crates/of_core/src/lib.rs:853`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L853) | `present` |
| `fn` | `sample_count` | Returns the number of samples tracked | [`crates/of_core/src/lib.rs:867`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L867) | `present` |
| `fn` | `reset` | Clears all samples | [`crates/of_core/src/lib.rs:872`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L872) | `present` |
| `struct` | `ResiliencySnapshot` | A snapshot of book resiliency metrics for the most recent trade | [`crates/of_core/src/lib.rs:880`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L880) | `present` |
| `enum` | `ClassificationVote` | Result of a single trade classification method | [`crates/of_core/src/lib.rs:898`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L898) | `present` |
| `struct` | `TradeClassifier` | Classifies trades using multiple methods: tick rule, quote rule, Lee-Ready, and consensus | [`crates/of_core/src/lib.rs:912`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L912) | `present` |
| `struct` | `ClassifierWeights` | Weights for the consensus voting classifier | [`crates/of_core/src/lib.rs:923`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L923) | `present` |
| `fn` | `new` | Creates a new classifier with default weights | [`crates/of_core/src/lib.rs:950`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L950) | `present` |
| `fn` | `with_weights` | Creates a classifier with custom weights | [`crates/of_core/src/lib.rs:959`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L959) | `present` |
| `fn` | `tick_rule` | Classifies a trade by tick rule based on price vs last price | [`crates/of_core/src/lib.rs:971`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L971) | `present` |
| `fn` | `quote_rule` | Classifies a trade by quote rule (compare to bid/ask) | [`crates/of_core/src/lib.rs:989`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L989) | `present` |
| `fn` | `lee_ready` | Classifies using Lee-Ready: quote rule at bid/ask, tick rule at mid | [`crates/of_core/src/lib.rs:1000`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1000) | `present` |
| `fn` | `classify` | Returns the consensus classification by weighted majority vote across all methods | [`crates/of_core/src/lib.rs:1024`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1024) | `present` |
| `fn` | `last_votes` | Returns the last votes for debug/diagnostics | [`crates/of_core/src/lib.rs:1070`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1070) | `present` |
| `fn` | `reset` | Resets the classifier state | [`crates/of_core/src/lib.rs:1075`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1075) | `present` |
| `struct` | `VpinSnapshot` | A single VPIN snapshot | [`crates/of_core/src/lib.rs:1084`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1084) | `present` |
| `struct` | `VpinTracker` | Tracks Volume-Synchronized Probability of Informed Trading (VPIN) | [`crates/of_core/src/lib.rs:1118`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1118) | `present` |
| `fn` | `new` | Creates a new VPIN tracker with specified bucket volume and rolling window size | [`crates/of_core/src/lib.rs:1135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1135) | `present` |
| `fn` | `with_toxicity_threshold` | Sets the toxicity threshold (z-score) | [`crates/of_core/src/lib.rs:1147`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1147) | `present` |
| `fn` | `on_trade` | Feeds classified volumes into the VPIN tracker | [`crates/of_core/src/lib.rs:1156`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1156) | `present` |
| `fn` | `snapshot` | Returns the current VPIN snapshot | [`crates/of_core/src/lib.rs:1179`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1179) | `present` |
| `fn` | `reset` | Resets all state | [`crates/of_core/src/lib.rs:1211`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1211) | `present` |
| `struct` | `KyleLambdaTracker` | Tracks Kyle's Lambda: `ΔP = α + λ * signed_volume + ε` over a rolling window | [`crates/of_core/src/lib.rs:1222`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1222) | `present` |
| `struct` | `KyleLambdaSnapshot` | Snapshot of Kyle's Lambda estimation | [`crates/of_core/src/lib.rs:1232`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1232) | `present` |
| `fn` | `new` | Creates a tracker that retains up to `window` samples | [`crates/of_core/src/lib.rs:1256`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1256) | `present` |
| `fn` | `on_trade` | Records a trade: signed volume (positive = buy) and price change | [`crates/of_core/src/lib.rs:1264`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1264) | `present` |
| `fn` | `snapshot` | Computes λ via OLS: `λ = cov(x,y) / var(x)`, α = mean(y) - λ * mean(x) | [`crates/of_core/src/lib.rs:1274`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1274) | `present` |
| `fn` | `reset` | Clears all recorded samples | [`crates/of_core/src/lib.rs:1335`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1335) | `present` |
| `struct` | `AmihudTracker` | Tracks Amihud Illiquidity: `\|return\| / dollar_volume` per bar | [`crates/of_core/src/lib.rs:1342`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1342) | `present` |
| `struct` | `AmihudSnapshot` | Snapshot of Amihud illiquidity | [`crates/of_core/src/lib.rs:1358`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1358) | `present` |
| `fn` | `new` | Creates a tracker with a rolling `window` of bars | [`crates/of_core/src/lib.rs:1379`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1379) | `present` |
| `fn` | `on_bar` | Records a bar: close price, dollar volume, previous close | [`crates/of_core/src/lib.rs:1387`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1387) | `present` |
| `fn` | `snapshot` | Returns the current Amihud illiquidity snapshot | [`crates/of_core/src/lib.rs:1405`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1405) | `present` |
| `fn` | `reset` | Clears all recorded bars | [`crates/of_core/src/lib.rs:1434`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1434) | `present` |
| `struct` | `CvdEnhancements` | Tracks CVD (Cumulative Volume Delta) enhancements: ratio, z-score, divergence | [`crates/of_core/src/lib.rs:1441`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1441) | `present` |
| `struct` | `CvdEnhancementSnapshot` | Snapshot of CVD enhancement metrics | [`crates/of_core/src/lib.rs:1455`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1455) | `present` |
| `fn` | `new` | Creates a CVD enhancement tracker with the given rolling `window` | [`crates/of_core/src/lib.rs:1476`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1476) | `present` |
| `fn` | `on_bar` | Records a bar's worth of delta, volume, and close price | [`crates/of_core/src/lib.rs:1486`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1486) | `present` |
| `fn` | `snapshot` | Returns current CVD enhancement metrics | [`crates/of_core/src/lib.rs:1501`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1501) | `present` |
| `fn` | `reset` | Clears all rolling CVD, volume, and price samples | [`crates/of_core/src/lib.rs:1547`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1547) | `present` |
| `struct` | `PatternSnapshot` | All detected practitioner patterns in one snapshot | [`crates/of_core/src/lib.rs:1558`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1558) | `present` |
| `struct` | `PatternDetector` | Detects practitioner orderflow patterns from book and trade data | [`crates/of_core/src/lib.rs:1658`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1658) | `present` |
| `fn` | `new` | Creates an empty pattern detector | [`crates/of_core/src/lib.rs:1719`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1719) | `present` |
| `fn` | `on_trade` | Feeds a trade into the detector | [`crates/of_core/src/lib.rs:1756`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1756) | `present` |
| `fn` | `on_book_update` | Feeds a book update for DOM/liquidity pattern detection | [`crates/of_core/src/lib.rs:1841`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1841) | `present` |
| `fn` | `snapshot` | Computes the current pattern snapshot | [`crates/of_core/src/lib.rs:1963`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1963) | `present` |
| `fn` | `reset` | Clears all detector state and rolling pattern history | [`crates/of_core/src/lib.rs:2337`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2337) | `present` |
| `enum` | `SignalState` | Output state emitted by signal modules | [`crates/of_core/src/lib.rs:2372`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2372) | `present` |
| `struct` | `SignalSnapshot` | Snapshot of a signal module evaluation | [`crates/of_core/src/lib.rs:2385`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2385) | `present` |
| `struct` | `DataQualityFlags` | Bitset wrapper for feed-quality flags | [`crates/of_core/src/lib.rs:2400`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2400) | `present` |
| `const` | `NONE` | No quality issues detected | [`crates/of_core/src/lib.rs:2404`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2404) | `present` |
| `const` | `STALE_FEED` | Feed is stale beyond policy threshold | [`crates/of_core/src/lib.rs:2406`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2406) | `present` |
| `const` | `SEQUENCE_GAP` | A sequence number gap was detected | [`crates/of_core/src/lib.rs:2408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2408) | `present` |
| `const` | `CLOCK_SKEW` | Clock skew detected between source and consumer | [`crates/of_core/src/lib.rs:2410`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2410) | `present` |
| `const` | `DEPTH_TRUNCATED` | Book depth was truncated | [`crates/of_core/src/lib.rs:2412`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2412) | `present` |
| `const` | `OUT_OF_ORDER` | Event arrived out-of-order | [`crates/of_core/src/lib.rs:2414`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2414) | `present` |
| `const` | `ADAPTER_DEGRADED` | Adapter/external feed is degraded or reconnecting | [`crates/of_core/src/lib.rs:2416`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2416) | `present` |
| `fn` | `bits` | Returns raw bit representation | [`crates/of_core/src/lib.rs:2419`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2419) | `present` |
| `fn` | `from_bits_truncate` | Builds flags from raw bits, preserving unknown bits | [`crates/of_core/src/lib.rs:2424`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2424) | `present` |
| `fn` | `intersects` | Returns true when any flag in `other` is set in `self` | [`crates/of_core/src/lib.rs:2429`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2429) | `present` |
| `struct` | `AnalyticsAccumulator` | In-memory accumulator that updates analytics state from normalized trades | [`crates/of_core/src/lib.rs:2450`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2450) | `present` |
| `fn` | `on_trade` | Applies a trade print to analytics and recomputes profile levels | [`crates/of_core/src/lib.rs:2484`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2484) | `present` |
| `fn` | `reset_session_delta` | Resets session delta and directional volume, keeps cumulative profile | [`crates/of_core/src/lib.rs:2534`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2534) | `present` |
| `fn` | `reset_session` | Resets all session analytics and volume-profile state | [`crates/of_core/src/lib.rs:2545`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2545) | `present` |
| `fn` | `snapshot` | Returns a copy of current analytics state | [`crates/of_core/src/lib.rs:2555`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2555) | `present` |
| `fn` | `derived_snapshot` | Returns additive derived analytics for the current session accumulator state | [`crates/of_core/src/lib.rs:2560`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2560) | `present` |
| `fn` | `session_candle_snapshot` | Returns candle-style session summary for the current analytics session | [`crates/of_core/src/lib.rs:2587`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2587) | `present` |
| `fn` | `interval_candle_snapshot` | Returns candle-style summary for trades observed inside a rolling interval | [`crates/of_core/src/lib.rs:2592`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2592) | `present` |
| `fn` | `with_tickbar` | Creates an accumulator with a tickbar aggregator at the given interval | [`crates/of_core/src/lib.rs:2645`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2645) | `present` |
| `fn` | `bar_series` | Returns completed bars from the tickbar aggregator and resets for continued collection | [`crates/of_core/src/lib.rs:2661`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2661) | `present` |
| `fn` | `reset_tickbar` | Removes the tickbar aggregator, freeing associated state | [`crates/of_core/src/lib.rs:2694`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2694) | `present` |
| `struct` | `VolatilitySnapshot` | Realised volatility estimators: Classic, Parkinson, Garman-Klass, Yang-Zhang | [`crates/of_core/src/lib.rs:2762`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2762) | `present` |
| `struct` | `VolatilityEstimator` | Tracks OHLC prices per bar for volatility estimation | [`crates/of_core/src/lib.rs:2786`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2786) | `present` |
| `fn` | `new` | Creates an estimator retaining up to `max_bars` OHLC bars | [`crates/of_core/src/lib.rs:2793`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2793) | `present` |
| `fn` | `on_bar` | Records one OHLC bar for volatility estimation | [`crates/of_core/src/lib.rs:2800`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2800) | `present` |
| `fn` | `snapshot` | Returns current realised-volatility estimates | [`crates/of_core/src/lib.rs:2804`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2804) | `present` |
| `struct` | `NoiseSnapshot` | Microstructure noise estimate and signal-to-noise ratio | [`crates/of_core/src/lib.rs:2841`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2841) | `present` |
| `struct` | `MicrostructureNoise` | Tracks price returns for noise estimation | [`crates/of_core/src/lib.rs:2859`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2859) | `present` |
| `fn` | `new` | Creates a noise estimator retaining up to `max_len` returns | [`crates/of_core/src/lib.rs:2867`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2867) | `present` |
| `fn` | `on_trade` | Records a trade price for return/noise estimation | [`crates/of_core/src/lib.rs:2875`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2875) | `present` |
| `fn` | `snapshot` | Returns the current microstructure-noise snapshot | [`crates/of_core/src/lib.rs:2884`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2884) | `present` |
| `struct` | `HasbrouckSnapshot` | Hasbrouck bivariate VAR(1) for price impact decomposition | [`crates/of_core/src/lib.rs:2916`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2916) | `present` |
| `struct` | `HasbrouckVAR` | Tracks returns and signed volume for Hasbrouck VAR | [`crates/of_core/src/lib.rs:2937`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2937) | `present` |
| `fn` | `new` | Creates a VAR estimator retaining up to `max_len` samples | [`crates/of_core/src/lib.rs:2945`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2945) | `present` |
| `fn` | `on_trade` | Records one return and signed-volume sample | [`crates/of_core/src/lib.rs:2953`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2953) | `present` |
| `fn` | `snapshot` | Returns current Hasbrouck impact estimates | [`crates/of_core/src/lib.rs:2963`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2963) | `present` |
| `struct` | `AlmgrenChrissSnapshot` | Almgren-Chriss market impact model | [`crates/of_core/src/lib.rs:3014`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3014) | `present` |
| `struct` | `AlmgrenChriss` | Tracks volume and price changes for impact estimation | [`crates/of_core/src/lib.rs:3032`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3032) | `present` |
| `fn` | `new` | Creates an impact estimator retaining up to `max_len` samples | [`crates/of_core/src/lib.rs:3040`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3040) | `present` |
| `fn` | `on_trade` | Records one price-change and signed-volume sample | [`crates/of_core/src/lib.rs:3048`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3048) | `present` |
| `fn` | `snapshot` | Returns current Almgren-Chriss impact estimates | [`crates/of_core/src/lib.rs:3058`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3058) | `present` |
| `struct` | `SpreadDecompositionSnapshot` | Huang-Stoll spread decomposition | [`crates/of_core/src/lib.rs:3085`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3085) | `present` |
| `struct` | `SpreadDecomposition` | Tracks spreads for Huang-Stoll decomposition | [`crates/of_core/src/lib.rs:3109`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3109) | `present` |
| `fn` | `new` | Creates a spread decomposition tracker retaining up to `max_len` samples | [`crates/of_core/src/lib.rs:3118`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3118) | `present` |
| `fn` | `on_spread` | Records effective, realised, and quoted spread observations | [`crates/of_core/src/lib.rs:3127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3127) | `present` |
| `fn` | `snapshot` | Returns current spread decomposition metrics | [`crates/of_core/src/lib.rs:3141`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3141) | `present` |
| `struct` | `ACDSnapshot` | ACD(1,1) model for trade duration | [`crates/of_core/src/lib.rs:3165`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3165) | `present` |
| `struct` | `ACDModel` | Tracks trade durations and estimates ACD(1,1) | [`crates/of_core/src/lib.rs:3189`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3189) | `present` |
| `fn` | `new` | Creates an ACD estimator retaining up to `max_len` durations | [`crates/of_core/src/lib.rs:3196`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3196) | `present` |
| `fn` | `on_trade` | Records a trade timestamp and previous trade timestamp | [`crates/of_core/src/lib.rs:3203`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3203) | `present` |
| `fn` | `snapshot` | Returns current ACD model estimates | [`crates/of_core/src/lib.rs:3210`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3210) | `present` |
| `enum` | `Regime` | Market regime classification | [`crates/of_core/src/lib.rs:3255`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3255) | `present` |
| `struct` | `RegimeSnapshot` | Snapshot of market-regime classification metrics | [`crates/of_core/src/lib.rs:3269`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3269) | `present` |
| `struct` | `RegimeDetector` | Classifies market regime from spread, volatility, and VPIN | [`crates/of_core/src/lib.rs:3293`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3293) | `present` |
| `fn` | `new` | Creates a regime detector retaining up to `max_len` samples | [`crates/of_core/src/lib.rs:3302`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3302) | `present` |
| `fn` | `on_metrics` | Records spread, volatility, and VPIN metrics | [`crates/of_core/src/lib.rs:3311`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3311) | `present` |
| `fn` | `snapshot` | Returns the current regime classification | [`crates/of_core/src/lib.rs:3325`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3325) | `present` |
| `struct` | `KineticEnergySnapshot` | Kinetic energy of order book activity | [`crates/of_core/src/lib.rs:3363`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3363) | `present` |
| `struct` | `KineticEnergyTracker` | Tracks book changes to compute kinetic energy analogues | [`crates/of_core/src/lib.rs:3384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3384) | `present` |
| `fn` | `new` | Creates a kinetic-energy tracker retaining up to `max_len` observations | [`crates/of_core/src/lib.rs:3391`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3391) | `present` |
| `fn` | `on_book_event` | Feeds a book update with relative level and velocity (size change) | [`crates/of_core/src/lib.rs:3398`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3398) | `present` |
| `fn` | `snapshot` | Returns current kinetic-energy metrics | [`crates/of_core/src/lib.rs:3408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3408) | `present` |
| `struct` | `DarkPoolSnapshot` | Dark pool analytics snapshot | [`crates/of_core/src/lib.rs:3433`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3433) | `present` |
| `struct` | `DarkPoolTracker` | Tracks dark pool volume alongside lit volume | [`crates/of_core/src/lib.rs:3454`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3454) | `present` |
| `fn` | `new` | Creates a dark-pool tracker retaining up to `max_days` observations | [`crates/of_core/src/lib.rs:3462`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3462) | `present` |
| `fn` | `on_day` | Records daily dark and lit volume | [`crates/of_core/src/lib.rs:3470`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3470) | `present` |
| `fn` | `snapshot` | Returns current dark-pool analytics | [`crates/of_core/src/lib.rs:3480`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3480) | `present` |
| `struct` | `OptionsFlowSnapshot` | Options flow snapshot | [`crates/of_core/src/lib.rs:3520`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3520) | `present` |
| `struct` | `OptionsFlowTracker` | Tracks options trade flow | [`crates/of_core/src/lib.rs:3544`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3544) | `present` |
| `fn` | `new` | Creates an options-flow tracker retaining up to `max_len` trades | [`crates/of_core/src/lib.rs:3558`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3558) | `present` |
| `fn` | `on_trade` | Records an options trade observation | [`crates/of_core/src/lib.rs:3565`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3565) | `present` |
| `fn` | `snapshot` | Returns current options-flow metrics | [`crates/of_core/src/lib.rs:3577`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3577) | `present` |
| `struct` | `FuturesSnapshot` | Futures analytics snapshot | [`crates/of_core/src/lib.rs:3608`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3608) | `present` |
| `struct` | `VolatilitySignaturePoint` | Volatility signature result at a specific lag | [`crates/of_core/src/lib.rs:3637`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3637) | `present` |
| `struct` | `VolatilitySignatureSnapshot` | Volatility signature plot: RV at multiple lags | [`crates/of_core/src/lib.rs:3647`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3647) | `present` |
| `struct` | `VolatilitySignature` | Computes volatility signature from return series | [`crates/of_core/src/lib.rs:3668`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3668) | `present` |
| `fn` | `new` | Creates a volatility-signature tracker retaining up to `max_len` returns | [`crates/of_core/src/lib.rs:3675`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3675) | `present` |
| `fn` | `on_return` | Records a return sample | [`crates/of_core/src/lib.rs:3682`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3682) | `present` |
| `fn` | `snapshot` | Returns the current volatility-signature snapshot | [`crates/of_core/src/lib.rs:3686`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3686) | `present` |
| `struct` | `AgentTypeSnapshot` | Agent-type identification snapshot | [`crates/of_core/src/lib.rs:3720`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3720) | `present` |
| `struct` | `AgentTypeDetector` | Infers agent types from trade and book patterns | [`crates/of_core/src/lib.rs:3744`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3744) | `present` |
| `fn` | `new` | Creates an agent-type detector retaining up to `max_len` observations | [`crates/of_core/src/lib.rs:3753`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3753) | `present` |
| `fn` | `on_event` | Records trade size and book-event rates for agent inference | [`crates/of_core/src/lib.rs:3762`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3762) | `present` |
| `fn` | `snapshot` | Returns current agent-type metrics | [`crates/of_core/src/lib.rs:3777`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3777) | `present` |
| `struct` | `LOBFeatureSnapshot` | 40+ hand-crafted LOB features for ML models | [`crates/of_core/src/lib.rs:3804`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3804) | `present` |
| `fn` | `compute_lob_features` | Computes LOB features from book snapshot and trade flow | [`crates/of_core/src/lib.rs:3863`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3863) | `present` |
| `struct` | `DarkLitCorrelationSnapshot` | Snapshot of dark-lit imbalance correlation | [`crates/of_core/src/lib.rs:3945`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3945) | `present` |
| `struct` | `DarkLitCorrelator` | Tracks rolling dark-lit imbalance correlation | [`crates/of_core/src/lib.rs:3963`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3963) | `present` |
| `fn` | `new` | Creates a correlator retaining up to `max_len` imbalance pairs | [`crates/of_core/src/lib.rs:3971`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3971) | `present` |
| `fn` | `on_imbalance` | Records one dark and lit imbalance pair | [`crates/of_core/src/lib.rs:3979`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3979) | `present` |
| `fn` | `snapshot` | Returns current dark-lit correlation metrics | [`crates/of_core/src/lib.rs:3989`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3989) | `present` |
| `struct` | `InstitutionalFlowSnapshot` | Snapshot of institutional-flow classification | [`crates/of_core/src/lib.rs:4022`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4022) | `present` |
| `struct` | `InstitutionalFlowTracker` | Tracks large trades for institutional-flow classification | [`crates/of_core/src/lib.rs:4040`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4040) | `present` |
| `fn` | `new` | Creates a tracker retaining up to `max_len` large trades | [`crates/of_core/src/lib.rs:4048`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4048) | `present` |
| `fn` | `on_trade` | Records one large trade and its inferred side | [`crates/of_core/src/lib.rs:4056`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4056) | `present` |
| `fn` | `snapshot` | Returns current institutional-flow metrics | [`crates/of_core/src/lib.rs:4068`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4068) | `present` |
| `struct` | `OIAnalysisSnapshot` | Snapshot of open-interest analysis | [`crates/of_core/src/lib.rs:4106`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4106) | `present` |
| `struct` | `OIAnalyzer` | Tracks open interest and price for divergence analysis | [`crates/of_core/src/lib.rs:4127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4127) | `present` |
| `fn` | `new` | Creates an analyzer retaining up to `max_len` observations | [`crates/of_core/src/lib.rs:4135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4135) | `present` |
| `fn` | `on_update` | Records one open-interest and price observation | [`crates/of_core/src/lib.rs:4143`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4143) | `present` |
| `fn` | `snapshot` | Returns current open-interest analysis metrics | [`crates/of_core/src/lib.rs:4155`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4155) | `present` |
| `struct` | `FXSnapshot` | Snapshot of FX-specific flow analytics | [`crates/of_core/src/lib.rs:4184`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4184) | `present` |
| `struct` | `FixedIncomeSnapshot` | Snapshot of fixed-income flow analytics | [`crates/of_core/src/lib.rs:4200`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4200) | `present` |
| `struct` | `CryptoSnapshot` | Snapshot of crypto-market flow analytics | [`crates/of_core/src/lib.rs:4219`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4219) | `present` |
| `struct` | `AlertRule` | Configurable real-time alert switches | [`crates/of_core/src/lib.rs:4245`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4245) | `present` |
| `trait` | `StateCheckpoint` | Trait for state serialization | [`crates/of_core/src/lib.rs:4263`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4263) | `present` |
| `struct` | `FuturesTracker` | Tracker for futures contract roll and basis | [`crates/of_core/src/lib.rs:4272`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4272) | `present` |
| `fn` | `new` | Creates a futures tracker retaining up to `max_len` ticks | [`crates/of_core/src/lib.rs:4282`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4282) | `present` |
| `fn` | `on_tick` | Records front/deferred prices and settlement volume context | [`crates/of_core/src/lib.rs:4292`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4292) | `present` |
| `fn` | `snapshot` | Returns current futures roll and basis metrics | [`crates/of_core/src/lib.rs:4314`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4314) | `present` |
| `struct` | `AnalyticsConfig` | Configurable analytics thresholds and buffer sizes | [`crates/of_core/src/lib.rs:4344`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4344) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `field` | `SymbolId` | `venue` | `: String` | [`crates/of_core/src/lib.rs:62`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L62) |
| `field` | `SymbolId` | `symbol` | `: String` | [`crates/of_core/src/lib.rs:64`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L64) |
| `variant` | `Side` | `Bid` | `Bid` | [`crates/of_core/src/lib.rs:71`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L71) |
| `variant` | `Side` | `Ask` | `Ask` | [`crates/of_core/src/lib.rs:73`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L73) |
| `variant` | `BookAction` | `Upsert` | `Upsert` | [`crates/of_core/src/lib.rs:80`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L80) |
| `variant` | `BookAction` | `Delete` | `Delete` | [`crates/of_core/src/lib.rs:82`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L82) |
| `field` | `BookUpdate` | `symbol` | `: SymbolId` | [`crates/of_core/src/lib.rs:89`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L89) |
| `field` | `BookUpdate` | `side` | `: Side` | [`crates/of_core/src/lib.rs:91`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L91) |
| `field` | `BookUpdate` | `level` | `: u16` | [`crates/of_core/src/lib.rs:93`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L93) |
| `field` | `BookUpdate` | `price` | `: i64` | [`crates/of_core/src/lib.rs:95`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L95) |
| `field` | `BookUpdate` | `size` | `: i64` | [`crates/of_core/src/lib.rs:97`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L97) |
| `field` | `BookUpdate` | `action` | `: BookAction` | [`crates/of_core/src/lib.rs:99`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L99) |
| `field` | `BookUpdate` | `sequence` | `: u64` | [`crates/of_core/src/lib.rs:101`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L101) |
| `field` | `BookUpdate` | `ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:103`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L103) |
| `field` | `BookUpdate` | `ts_recv_ns` | `: u64` | [`crates/of_core/src/lib.rs:105`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L105) |
| `field` | `BookLevel` | `level` | `: u16` | [`crates/of_core/src/lib.rs:112`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L112) |
| `field` | `BookLevel` | `price` | `: i64` | [`crates/of_core/src/lib.rs:114`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L114) |
| `field` | `BookLevel` | `size` | `: i64` | [`crates/of_core/src/lib.rs:116`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L116) |
| `field` | `BookSnapshot` | `symbol` | `: SymbolId` | [`crates/of_core/src/lib.rs:123`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L123) |
| `field` | `BookSnapshot` | `bids` | `: Vec<BookLevel>` | [`crates/of_core/src/lib.rs:125`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L125) |
| `field` | `BookSnapshot` | `asks` | `: Vec<BookLevel>` | [`crates/of_core/src/lib.rs:127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L127) |
| `field` | `BookSnapshot` | `last_sequence` | `: u64` | [`crates/of_core/src/lib.rs:129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L129) |
| `field` | `BookSnapshot` | `ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:131`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L131) |
| `field` | `BookSnapshot` | `ts_recv_ns` | `: u64` | [`crates/of_core/src/lib.rs:133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L133) |
| `field` | `TradePrint` | `symbol` | `: SymbolId` | [`crates/of_core/src/lib.rs:140`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L140) |
| `field` | `TradePrint` | `price` | `: i64` | [`crates/of_core/src/lib.rs:142`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L142) |
| `field` | `TradePrint` | `size` | `: i64` | [`crates/of_core/src/lib.rs:144`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L144) |
| `field` | `TradePrint` | `aggressor_side` | `: Side` | [`crates/of_core/src/lib.rs:146`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L146) |
| `field` | `TradePrint` | `sequence` | `: u64` | [`crates/of_core/src/lib.rs:148`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L148) |
| `field` | `TradePrint` | `ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:150`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L150) |
| `field` | `TradePrint` | `ts_recv_ns` | `: u64` | [`crates/of_core/src/lib.rs:152`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L152) |
| `field` | `AnalyticsSnapshot` | `delta` | `: i64` | [`crates/of_core/src/lib.rs:159`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L159) |
| `field` | `AnalyticsSnapshot` | `cumulative_delta` | `: i64` | [`crates/of_core/src/lib.rs:161`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L161) |
| `field` | `AnalyticsSnapshot` | `buy_volume` | `: i64` | [`crates/of_core/src/lib.rs:163`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L163) |
| `field` | `AnalyticsSnapshot` | `sell_volume` | `: i64` | [`crates/of_core/src/lib.rs:165`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L165) |
| `field` | `AnalyticsSnapshot` | `last_price` | `: i64` | [`crates/of_core/src/lib.rs:167`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L167) |
| `field` | `AnalyticsSnapshot` | `point_of_control` | `: i64` | [`crates/of_core/src/lib.rs:169`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L169) |
| `field` | `AnalyticsSnapshot` | `value_area_low` | `: i64` | [`crates/of_core/src/lib.rs:171`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L171) |
| `field` | `AnalyticsSnapshot` | `value_area_high` | `: i64` | [`crates/of_core/src/lib.rs:173`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L173) |
| `field` | `DerivedAnalyticsSnapshot` | `total_volume` | `: i64` | [`crates/of_core/src/lib.rs:180`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L180) |
| `field` | `DerivedAnalyticsSnapshot` | `trade_count` | `: u64` | [`crates/of_core/src/lib.rs:182`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L182) |
| `field` | `DerivedAnalyticsSnapshot` | `vwap` | `: i64` | [`crates/of_core/src/lib.rs:184`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L184) |
| `field` | `DerivedAnalyticsSnapshot` | `average_trade_size` | `: i64` | [`crates/of_core/src/lib.rs:186`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L186) |
| `field` | `DerivedAnalyticsSnapshot` | `imbalance_bps` | `: i64` | [`crates/of_core/src/lib.rs:188`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L188) |
| `field` | `SessionCandleSnapshot` | `open` | `: i64` | [`crates/of_core/src/lib.rs:195`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L195) |
| `field` | `SessionCandleSnapshot` | `high` | `: i64` | [`crates/of_core/src/lib.rs:197`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L197) |
| `field` | `SessionCandleSnapshot` | `low` | `: i64` | [`crates/of_core/src/lib.rs:199`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L199) |
| `field` | `SessionCandleSnapshot` | `close` | `: i64` | [`crates/of_core/src/lib.rs:201`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L201) |
| `field` | `SessionCandleSnapshot` | `trade_count` | `: u64` | [`crates/of_core/src/lib.rs:203`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L203) |
| `field` | `SessionCandleSnapshot` | `first_ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:205`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L205) |
| `field` | `SessionCandleSnapshot` | `last_ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:207`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L207) |
| `field` | `IntervalCandleSnapshot` | `window_ns` | `: u64` | [`crates/of_core/src/lib.rs:214`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L214) |
| `field` | `IntervalCandleSnapshot` | `open` | `: i64` | [`crates/of_core/src/lib.rs:216`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L216) |
| `field` | `IntervalCandleSnapshot` | `high` | `: i64` | [`crates/of_core/src/lib.rs:218`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L218) |
| `field` | `IntervalCandleSnapshot` | `low` | `: i64` | [`crates/of_core/src/lib.rs:220`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L220) |
| `field` | `IntervalCandleSnapshot` | `close` | `: i64` | [`crates/of_core/src/lib.rs:222`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L222) |
| `field` | `IntervalCandleSnapshot` | `trade_count` | `: u64` | [`crates/of_core/src/lib.rs:224`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L224) |
| `field` | `IntervalCandleSnapshot` | `total_volume` | `: i64` | [`crates/of_core/src/lib.rs:226`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L226) |
| `field` | `IntervalCandleSnapshot` | `vwap` | `: i64` | [`crates/of_core/src/lib.rs:228`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L228) |
| `field` | `IntervalCandleSnapshot` | `first_ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:230`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L230) |
| `field` | `IntervalCandleSnapshot` | `last_ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:232`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L232) |
| `field` | `CompletedBar` | `timestamp_ns` | `: i64` | [`crates/of_core/src/lib.rs:239`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L239) |
| `field` | `CompletedBar` | `open` | `: i64` | [`crates/of_core/src/lib.rs:241`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L241) |
| `field` | `CompletedBar` | `high` | `: i64` | [`crates/of_core/src/lib.rs:243`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L243) |
| `field` | `CompletedBar` | `low` | `: i64` | [`crates/of_core/src/lib.rs:245`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L245) |
| `field` | `CompletedBar` | `close` | `: i64` | [`crates/of_core/src/lib.rs:247`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L247) |
| `field` | `CompletedBar` | `volume` | `: i64` | [`crates/of_core/src/lib.rs:249`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L249) |
| `field` | `CompletedBar` | `tick_count` | `: u64` | [`crates/of_core/src/lib.rs:251`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L251) |
| `field` | `CompletedBar` | `vwap` | `: i64` | [`crates/of_core/src/lib.rs:253`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L253) |
| `field` | `BookAnalyticsSnapshot` | `best_bid` | `: i64` | [`crates/of_core/src/lib.rs:260`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L260) |
| `field` | `BookAnalyticsSnapshot` | `best_ask` | `: i64` | [`crates/of_core/src/lib.rs:262`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L262) |
| `field` | `BookAnalyticsSnapshot` | `quoted_spread` | `: i64` | [`crates/of_core/src/lib.rs:264`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L264) |
| `field` | `BookAnalyticsSnapshot` | `relative_spread_bps` | `: i64` | [`crates/of_core/src/lib.rs:266`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L266) |
| `field` | `BookAnalyticsSnapshot` | `microprice` | `: i64` | [`crates/of_core/src/lib.rs:268`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L268) |
| `field` | `BookAnalyticsSnapshot` | `bid_depth` | `: i64` | [`crates/of_core/src/lib.rs:270`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L270) |
| `field` | `BookAnalyticsSnapshot` | `ask_depth` | `: i64` | [`crates/of_core/src/lib.rs:272`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L272) |
| `field` | `BookAnalyticsSnapshot` | `depth_imbalance_bps` | `: i64` | [`crates/of_core/src/lib.rs:275`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L275) |
| `field` | `SpreadSample` | `trade_price` | `: i64` | [`crates/of_core/src/lib.rs:467`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L467) |
| `field` | `SpreadSample` | `mid_price` | `: i64` | [`crates/of_core/src/lib.rs:469`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L469) |
| `field` | `SpreadSample` | `ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:471`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L471) |
| `field` | `BookEventSample` | `side` | `: Side` | [`crates/of_core/src/lib.rs:559`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L559) |
| `field` | `BookEventSample` | `action` | `: BookAction` | [`crates/of_core/src/lib.rs:561`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L561) |
| `field` | `BookEventSample` | `size` | `: i64` | [`crates/of_core/src/lib.rs:563`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L563) |
| `field` | `BookEventSample` | `ts_exchange_ns` | `: u64` | [`crates/of_core/src/lib.rs:565`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L565) |
| `field` | `BookEventAnalyticsSnapshot` | `bid_arrival_rate` | `: f64` | [`crates/of_core/src/lib.rs:709`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L709) |
| `field` | `BookEventAnalyticsSnapshot` | `ask_arrival_rate` | `: f64` | [`crates/of_core/src/lib.rs:711`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L711) |
| `field` | `BookEventAnalyticsSnapshot` | `bid_cancel_rate` | `: f64` | [`crates/of_core/src/lib.rs:713`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L713) |
| `field` | `BookEventAnalyticsSnapshot` | `ask_cancel_rate` | `: f64` | [`crates/of_core/src/lib.rs:715`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L715) |
| `field` | `BookEventAnalyticsSnapshot` | `change_intensity` | `: f64` | [`crates/of_core/src/lib.rs:717`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L717) |
| `field` | `BookEventAnalyticsSnapshot` | `bid_event_volume` | `: i64` | [`crates/of_core/src/lib.rs:719`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L719) |
| `field` | `BookEventAnalyticsSnapshot` | `ask_event_volume` | `: i64` | [`crates/of_core/src/lib.rs:721`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L721) |
| `field` | `ResiliencySample` | `pre_bid_depth` | `: i64` | [`crates/of_core/src/lib.rs:764`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L764) |
| `field` | `ResiliencySample` | `pre_ask_depth` | `: i64` | [`crates/of_core/src/lib.rs:766`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L766) |
| `field` | `ResiliencySample` | `post_ts` | `: u64` | [`crates/of_core/src/lib.rs:768`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L768) |
| `field` | `ResiliencySample` | `post_bid_depth` | `: i64` | [`crates/of_core/src/lib.rs:770`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L770) |
| `field` | `ResiliencySample` | `post_ask_depth` | `: i64` | [`crates/of_core/src/lib.rs:772`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L772) |
| `field` | `ResiliencySample` | `recovery_ts` | `: u64` | [`crates/of_core/src/lib.rs:774`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L774) |
| `field` | `ResiliencySnapshot` | `recovery_time_ms` | `: f64` | [`crates/of_core/src/lib.rs:882`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L882) |
| `field` | `ResiliencySnapshot` | `depth_elasticity` | `: f64` | [`crates/of_core/src/lib.rs:884`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L884) |
| `variant` | `ClassificationVote` | `Buy` | `Buy` | [`crates/of_core/src/lib.rs:900`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L900) |
| `variant` | `ClassificationVote` | `Sell` | `Sell` | [`crates/of_core/src/lib.rs:902`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L902) |
| `variant` | `ClassificationVote` | `Neutral` | `Neutral` | [`crates/of_core/src/lib.rs:904`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L904) |
| `field` | `ClassifierWeights` | `tick_weight` | `: f64` | [`crates/of_core/src/lib.rs:925`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L925) |
| `field` | `ClassifierWeights` | `quote_weight` | `: f64` | [`crates/of_core/src/lib.rs:927`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L927) |
| `field` | `ClassifierWeights` | `lee_ready_weight` | `: f64` | [`crates/of_core/src/lib.rs:929`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L929) |
| `field` | `VpinSnapshot` | `vpin` | `: f64` | [`crates/of_core/src/lib.rs:1086`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1086) |
| `field` | `VpinSnapshot` | `vpin_zscore` | `: f64` | [`crates/of_core/src/lib.rs:1088`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1088) |
| `field` | `VpinSnapshot` | `vpin_mean` | `: f64` | [`crates/of_core/src/lib.rs:1090`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1090) |
| `field` | `VpinSnapshot` | `vpin_std` | `: f64` | [`crates/of_core/src/lib.rs:1092`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1092) |
| `field` | `VpinSnapshot` | `is_toxic` | `: bool` | [`crates/of_core/src/lib.rs:1094`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1094) |
| `field` | `VpinSnapshot` | `bucket_count` | `: u64` | [`crates/of_core/src/lib.rs:1096`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1096) |
| `field` | `KyleLambdaSnapshot` | `lambda_bps` | `: f64` | [`crates/of_core/src/lib.rs:1234`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1234) |
| `field` | `KyleLambdaSnapshot` | `r_squared` | `: f64` | [`crates/of_core/src/lib.rs:1236`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1236) |
| `field` | `KyleLambdaSnapshot` | `average_lambda_bps` | `: f64` | [`crates/of_core/src/lib.rs:1238`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1238) |
| `field` | `KyleLambdaSnapshot` | `sample_count` | `: u32` | [`crates/of_core/src/lib.rs:1240`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1240) |
| `field` | `AmihudSnapshot` | `amihud_ratio` | `: f64` | [`crates/of_core/src/lib.rs:1360`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1360) |
| `field` | `AmihudSnapshot` | `average_illiquidity` | `: f64` | [`crates/of_core/src/lib.rs:1362`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1362) |
| `field` | `AmihudSnapshot` | `bar_count` | `: u32` | [`crates/of_core/src/lib.rs:1364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1364) |
| `field` | `CvdEnhancementSnapshot` | `delta_ratio` | `: f64` | [`crates/of_core/src/lib.rs:1457`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1457) |
| `field` | `CvdEnhancementSnapshot` | `delta_zscore` | `: f64` | [`crates/of_core/src/lib.rs:1459`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1459) |
| `field` | `CvdEnhancementSnapshot` | `divergence_detected` | `: bool` | [`crates/of_core/src/lib.rs:1461`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1461) |
| `field` | `PatternSnapshot` | `imbalance_detected` | `: bool` | [`crates/of_core/src/lib.rs:1560`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1560) |
| `field` | `PatternSnapshot` | `stacked_imbalance_detected` | `: bool` | [`crates/of_core/src/lib.rs:1562`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1562) |
| `field` | `PatternSnapshot` | `absorption_detected` | `: bool` | [`crates/of_core/src/lib.rs:1564`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1564) |
| `field` | `PatternSnapshot` | `exhaustion_detected` | `: bool` | [`crates/of_core/src/lib.rs:1566`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1566) |
| `field` | `PatternSnapshot` | `initiation_detected` | `: bool` | [`crates/of_core/src/lib.rs:1568`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1568) |
| `field` | `PatternSnapshot` | `tailing_detected` | `: bool` | [`crates/of_core/src/lib.rs:1570`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1570) |
| `field` | `PatternSnapshot` | `iceberg_detected` | `: bool` | [`crates/of_core/src/lib.rs:1572`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1572) |
| `field` | `PatternSnapshot` | `spoofing_detected` | `: bool` | [`crates/of_core/src/lib.rs:1574`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1574) |
| `field` | `PatternSnapshot` | `flip_detected` | `: bool` | [`crates/of_core/src/lib.rs:1576`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1576) |
| `field` | `PatternSnapshot` | `liquidity_gap_detected` | `: bool` | [`crates/of_core/src/lib.rs:1578`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1578) |
| `field` | `PatternSnapshot` | `stop_hunt_detected` | `: bool` | [`crates/of_core/src/lib.rs:1580`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1580) |
| `field` | `PatternSnapshot` | `hidden_accumulation` | `: bool` | [`crates/of_core/src/lib.rs:1582`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1582) |
| `field` | `PatternSnapshot` | `hidden_distribution` | `: bool` | [`crates/of_core/src/lib.rs:1584`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1584) |
| `field` | `PatternSnapshot` | `trapped_traders_detected` | `: bool` | [`crates/of_core/src/lib.rs:1586`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1586) |
| `field` | `PatternSnapshot` | `delta_clock_ns` | `: u64` | [`crates/of_core/src/lib.rs:1588`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1588) |
| `field` | `PatternSnapshot` | `trend_day` | `: bool` | [`crates/of_core/src/lib.rs:1590`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1590) |
| `field` | `PatternSnapshot` | `range_day` | `: bool` | [`crates/of_core/src/lib.rs:1592`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1592) |
| `field` | `PatternSnapshot` | `reversal_day` | `: bool` | [`crates/of_core/src/lib.rs:1594`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1594) |
| `field` | `PatternSnapshot` | `session_type_score` | `: f64` | [`crates/of_core/src/lib.rs:1596`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1596) |
| `field` | `PatternSnapshot` | `volume_entropy` | `: f64` | [`crates/of_core/src/lib.rs:1599`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1599) |
| `field` | `PatternSnapshot` | `volume_skew` | `: f64` | [`crates/of_core/src/lib.rs:1601`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1601) |
| `field` | `PatternSnapshot` | `initial_balance_high` | `: i64` | [`crates/of_core/src/lib.rs:1603`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1603) |
| `field` | `PatternSnapshot` | `initial_balance_low` | `: i64` | [`crates/of_core/src/lib.rs:1605`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1605) |
| `field` | `PatternSnapshot` | `hvn_count` | `: u32` | [`crates/of_core/src/lib.rs:1607`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1607) |
| `field` | `PatternSnapshot` | `lvn_count` | `: u32` | [`crates/of_core/src/lib.rs:1609`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1609) |
| `field` | `PatternSnapshot` | `vwap_per_bin_json` | `: [u8; 512]` | [`crates/of_core/src/lib.rs:1611`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1611) |
| `field` | `PatternSnapshot` | `composite_hvn` | `: u32` | [`crates/of_core/src/lib.rs:1613`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1613) |
| `field` | `PatternSnapshot` | `composite_lvn` | `: u32` | [`crates/of_core/src/lib.rs:1615`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L1615) |
| `variant` | `SignalState` | `Neutral` | `Neutral` | [`crates/of_core/src/lib.rs:2374`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2374) |
| `variant` | `SignalState` | `LongBias` | `LongBias` | [`crates/of_core/src/lib.rs:2376`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2376) |
| `variant` | `SignalState` | `ShortBias` | `ShortBias` | [`crates/of_core/src/lib.rs:2378`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2378) |
| `variant` | `SignalState` | `Blocked` | `Blocked` | [`crates/of_core/src/lib.rs:2380`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2380) |
| `field` | `SignalSnapshot` | `module_id` | `: &'static str` | [`crates/of_core/src/lib.rs:2387`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2387) |
| `field` | `SignalSnapshot` | `state` | `: SignalState` | [`crates/of_core/src/lib.rs:2389`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2389) |
| `field` | `SignalSnapshot` | `confidence_bps` | `: u16` | [`crates/of_core/src/lib.rs:2391`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2391) |
| `field` | `SignalSnapshot` | `quality_flags` | `: u32` | [`crates/of_core/src/lib.rs:2393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2393) |
| `field` | `SignalSnapshot` | `reason` | `: String` | [`crates/of_core/src/lib.rs:2395`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2395) |
| `const` | `-` | `NONE` | `: Self = Self(0)` | [`crates/of_core/src/lib.rs:2404`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2404) |
| `const` | `-` | `STALE_FEED` | `: Self = Self(1 << 0)` | [`crates/of_core/src/lib.rs:2406`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2406) |
| `const` | `-` | `SEQUENCE_GAP` | `: Self = Self(1 << 1)` | [`crates/of_core/src/lib.rs:2408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2408) |
| `const` | `-` | `CLOCK_SKEW` | `: Self = Self(1 << 2)` | [`crates/of_core/src/lib.rs:2410`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2410) |
| `const` | `-` | `DEPTH_TRUNCATED` | `: Self = Self(1 << 3)` | [`crates/of_core/src/lib.rs:2412`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2412) |
| `const` | `-` | `OUT_OF_ORDER` | `: Self = Self(1 << 4)` | [`crates/of_core/src/lib.rs:2414`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2414) |
| `const` | `-` | `ADAPTER_DEGRADED` | `: Self = Self(1 << 5)` | [`crates/of_core/src/lib.rs:2416`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2416) |
| `field` | `VolatilitySnapshot` | `classic_rv` | `: f64` | [`crates/of_core/src/lib.rs:2764`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2764) |
| `field` | `VolatilitySnapshot` | `parkinson` | `: f64` | [`crates/of_core/src/lib.rs:2766`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2766) |
| `field` | `VolatilitySnapshot` | `garman_klass` | `: f64` | [`crates/of_core/src/lib.rs:2768`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2768) |
| `field` | `VolatilitySnapshot` | `yang_zhang` | `: f64` | [`crates/of_core/src/lib.rs:2770`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2770) |
| `field` | `NoiseSnapshot` | `noise_variance` | `: f64` | [`crates/of_core/src/lib.rs:2843`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2843) |
| `field` | `NoiseSnapshot` | `signal_to_noise` | `: f64` | [`crates/of_core/src/lib.rs:2845`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2845) |
| `field` | `HasbrouckSnapshot` | `permanent_impact` | `: f64` | [`crates/of_core/src/lib.rs:2918`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2918) |
| `field` | `HasbrouckSnapshot` | `temporary_impact` | `: f64` | [`crates/of_core/src/lib.rs:2920`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2920) |
| `field` | `HasbrouckSnapshot` | `information_share` | `: f64` | [`crates/of_core/src/lib.rs:2922`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L2922) |
| `field` | `AlmgrenChrissSnapshot` | `permanent_impact_coef` | `: f64` | [`crates/of_core/src/lib.rs:3016`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3016) |
| `field` | `AlmgrenChrissSnapshot` | `temporary_impact_coef` | `: f64` | [`crates/of_core/src/lib.rs:3018`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3018) |
| `field` | `SpreadDecompositionSnapshot` | `adverse_selection` | `: f64` | [`crates/of_core/src/lib.rs:3087`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3087) |
| `field` | `SpreadDecompositionSnapshot` | `order_processing_cost` | `: f64` | [`crates/of_core/src/lib.rs:3089`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3089) |
| `field` | `SpreadDecompositionSnapshot` | `inventory_component` | `: f64` | [`crates/of_core/src/lib.rs:3091`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3091) |
| `field` | `SpreadDecompositionSnapshot` | `pin` | `: f64` | [`crates/of_core/src/lib.rs:3093`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3093) |
| `field` | `ACDSnapshot` | `mean_duration_ns` | `: f64` | [`crates/of_core/src/lib.rs:3167`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3167) |
| `field` | `ACDSnapshot` | `intensity` | `: f64` | [`crates/of_core/src/lib.rs:3169`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3169) |
| `field` | `ACDSnapshot` | `alpha` | `: f64` | [`crates/of_core/src/lib.rs:3171`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3171) |
| `field` | `ACDSnapshot` | `beta` | `: f64` | [`crates/of_core/src/lib.rs:3173`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3173) |
| `variant` | `Regime` | `Normal` | `Normal = 0` | [`crates/of_core/src/lib.rs:3257`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3257) |
| `variant` | `Regime` | `Stressed` | `Stressed = 1` | [`crates/of_core/src/lib.rs:3259`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3259) |
| `variant` | `Regime` | `FlashCrash` | `FlashCrash = 2` | [`crates/of_core/src/lib.rs:3261`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3261) |
| `variant` | `Regime` | `Quiet` | `Quiet = 3` | [`crates/of_core/src/lib.rs:3263`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3263) |
| `field` | `RegimeSnapshot` | `regime` | `: u32` | [`crates/of_core/src/lib.rs:3271`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3271) |
| `field` | `RegimeSnapshot` | `spread_z` | `: f64` | [`crates/of_core/src/lib.rs:3273`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3273) |
| `field` | `RegimeSnapshot` | `vol_z` | `: f64` | [`crates/of_core/src/lib.rs:3275`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3275) |
| `field` | `RegimeSnapshot` | `vpin_z` | `: f64` | [`crates/of_core/src/lib.rs:3277`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3277) |
| `field` | `KineticEnergySnapshot` | `kinetic_energy` | `: f64` | [`crates/of_core/src/lib.rs:3365`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3365) |
| `field` | `KineticEnergySnapshot` | `order_flow_momentum` | `: f64` | [`crates/of_core/src/lib.rs:3367`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3367) |
| `field` | `KineticEnergySnapshot` | `energy_change` | `: f64` | [`crates/of_core/src/lib.rs:3369`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3369) |
| `field` | `DarkPoolSnapshot` | `dark_volume_pct` | `: f64` | [`crates/of_core/src/lib.rs:3435`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3435) |
| `field` | `DarkPoolSnapshot` | `dark_zscore` | `: f64` | [`crates/of_core/src/lib.rs:3437`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3437) |
| `field` | `DarkPoolSnapshot` | `dark_lit_divergence` | `: bool` | [`crates/of_core/src/lib.rs:3439`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3439) |
| `field` | `OptionsFlowSnapshot` | `sweep_detected` | `: bool` | [`crates/of_core/src/lib.rs:3522`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3522) |
| `field` | `OptionsFlowSnapshot` | `put_call_ratio` | `: f64` | [`crates/of_core/src/lib.rs:3524`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3524) |
| `field` | `OptionsFlowSnapshot` | `delta_notional` | `: f64` | [`crates/of_core/src/lib.rs:3526`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3526) |
| `field` | `OptionsFlowSnapshot` | `gamma_positioning` | `: f64` | [`crates/of_core/src/lib.rs:3528`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3528) |
| `field` | `FuturesSnapshot` | `basis_bps` | `: f64` | [`crates/of_core/src/lib.rs:3610`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3610) |
| `field` | `FuturesSnapshot` | `calendar_spread` | `: f64` | [`crates/of_core/src/lib.rs:3612`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3612) |
| `field` | `FuturesSnapshot` | `settlement_pressure` | `: f64` | [`crates/of_core/src/lib.rs:3614`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3614) |
| `field` | `FuturesSnapshot` | `roll_progress` | `: f64` | [`crates/of_core/src/lib.rs:3616`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3616) |
| `field` | `VolatilitySignaturePoint` | `lag` | `: u32` | [`crates/of_core/src/lib.rs:3639`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3639) |
| `field` | `VolatilitySignaturePoint` | `rv` | `: f64` | [`crates/of_core/src/lib.rs:3641`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3641) |
| `field` | `VolatilitySignatureSnapshot` | `points` | `: [VolatilitySignaturePoint; 10]` | [`crates/of_core/src/lib.rs:3649`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3649) |
| `field` | `VolatilitySignatureSnapshot` | `point_count` | `: u32` | [`crates/of_core/src/lib.rs:3651`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3651) |
| `field` | `VolatilitySignatureSnapshot` | `optimal_lag` | `: u32` | [`crates/of_core/src/lib.rs:3653`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3653) |
| `field` | `AgentTypeSnapshot` | `irp` | `: f64` | [`crates/of_core/src/lib.rs:3722`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3722) |
| `field` | `AgentTypeSnapshot` | `ipin` | `: f64` | [`crates/of_core/src/lib.rs:3724`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3724) |
| `field` | `AgentTypeSnapshot` | `ivpin` | `: f64` | [`crates/of_core/src/lib.rs:3726`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3726) |
| `field` | `AgentTypeSnapshot` | `hft_reflexivity` | `: f64` | [`crates/of_core/src/lib.rs:3728`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3728) |
| `field` | `LOBFeatureSnapshot` | `spread_bps` | `: f64` | [`crates/of_core/src/lib.rs:3806`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3806) |
| `field` | `LOBFeatureSnapshot` | `depth_imbalance` | `: f64` | [`crates/of_core/src/lib.rs:3808`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3808) |
| `field` | `LOBFeatureSnapshot` | `microprice` | `: f64` | [`crates/of_core/src/lib.rs:3810`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3810) |
| `field` | `LOBFeatureSnapshot` | `depth_slope` | `: f64` | [`crates/of_core/src/lib.rs:3812`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3812) |
| `field` | `LOBFeatureSnapshot` | `order_intensity` | `: f64` | [`crates/of_core/src/lib.rs:3814`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3814) |
| `field` | `LOBFeatureSnapshot` | `price_pressure_1` | `: f64` | [`crates/of_core/src/lib.rs:3816`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3816) |
| `field` | `LOBFeatureSnapshot` | `price_pressure_5` | `: f64` | [`crates/of_core/src/lib.rs:3818`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3818) |
| `field` | `LOBFeatureSnapshot` | `price_pressure_10` | `: f64` | [`crates/of_core/src/lib.rs:3820`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3820) |
| `field` | `LOBFeatureSnapshot` | `bid_ask_ratio_1` | `: f64` | [`crates/of_core/src/lib.rs:3822`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3822) |
| `field` | `LOBFeatureSnapshot` | `bid_ask_ratio_5` | `: f64` | [`crates/of_core/src/lib.rs:3824`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3824) |
| `field` | `LOBFeatureSnapshot` | `bid_ask_ratio_10` | `: f64` | [`crates/of_core/src/lib.rs:3826`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3826) |
| `field` | `LOBFeatureSnapshot` | `weighted_spread` | `: f64` | [`crates/of_core/src/lib.rs:3828`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3828) |
| `field` | `LOBFeatureSnapshot` | `volume_concentration` | `: f64` | [`crates/of_core/src/lib.rs:3830`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3830) |
| `field` | `LOBFeatureSnapshot` | `cancel_intensity` | `: f64` | [`crates/of_core/src/lib.rs:3832`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3832) |
| `field` | `LOBFeatureSnapshot` | `arrival_intensity` | `: f64` | [`crates/of_core/src/lib.rs:3834`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3834) |
| `field` | `LOBFeatureSnapshot` | `trade_flow_imbalance` | `: f64` | [`crates/of_core/src/lib.rs:3836`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3836) |
| `field` | `DarkLitCorrelationSnapshot` | `correlation` | `: f64` | [`crates/of_core/src/lib.rs:3947`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3947) |
| `field` | `DarkLitCorrelationSnapshot` | `siphon_active` | `: bool` | [`crates/of_core/src/lib.rs:3949`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L3949) |
| `field` | `InstitutionalFlowSnapshot` | `institutional_buy_ratio` | `: f64` | [`crates/of_core/src/lib.rs:4024`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4024) |
| `field` | `InstitutionalFlowSnapshot` | `crowding_score` | `: f64` | [`crates/of_core/src/lib.rs:4026`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4026) |
| `field` | `OIAnalysisSnapshot` | `oi_divergence` | `: bool` | [`crates/of_core/src/lib.rs:4108`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4108) |
| `field` | `OIAnalysisSnapshot` | `oi_build_rate` | `: f64` | [`crates/of_core/src/lib.rs:4110`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4110) |
| `field` | `OIAnalysisSnapshot` | `max_pain_distance_bps` | `: f64` | [`crates/of_core/src/lib.rs:4112`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4112) |
| `field` | `FXSnapshot` | `cross_currency_correlation` | `: f64` | [`crates/of_core/src/lib.rs:4186`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4186) |
| `field` | `FixedIncomeSnapshot` | `yield_curve_positioning` | `: f64` | [`crates/of_core/src/lib.rs:4202`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4202) |
| `field` | `FixedIncomeSnapshot` | `duration_weighted_flow` | `: f64` | [`crates/of_core/src/lib.rs:4204`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4204) |
| `field` | `CryptoSnapshot` | `exchange_flow_balance` | `: f64` | [`crates/of_core/src/lib.rs:4221`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4221) |
| `field` | `CryptoSnapshot` | `funding_rate_basis` | `: f64` | [`crates/of_core/src/lib.rs:4223`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4223) |
| `field` | `CryptoSnapshot` | `wash_trading_score` | `: f64` | [`crates/of_core/src/lib.rs:4225`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4225) |
| `field` | `AlertRule` | `absorption_alert` | `: bool` | [`crates/of_core/src/lib.rs:4247`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4247) |
| `field` | `AlertRule` | `delta_divergence_alert` | `: bool` | [`crates/of_core/src/lib.rs:4249`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4249) |
| `field` | `AlertRule` | `stacked_imbalance_alert` | `: bool` | [`crates/of_core/src/lib.rs:4251`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4251) |
| `field` | `AlertRule` | `iceberg_alert` | `: bool` | [`crates/of_core/src/lib.rs:4253`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4253) |
| `field` | `AlertRule` | `vpin_toxic_alert` | `: bool` | [`crates/of_core/src/lib.rs:4255`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4255) |
| `field` | `AnalyticsConfig` | `vpin_volume_bucket` | `: i64` | [`crates/of_core/src/lib.rs:4346`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4346) |
| `field` | `AnalyticsConfig` | `vpin_max_buckets` | `: u32` | [`crates/of_core/src/lib.rs:4348`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4348) |
| `field` | `AnalyticsConfig` | `kyle_lambda_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4350`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4350) |
| `field` | `AnalyticsConfig` | `cvd_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4352`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4352) |
| `field` | `AnalyticsConfig` | `vol_estimator_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4354`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4354) |
| `field` | `AnalyticsConfig` | `noise_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4356`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4356) |
| `field` | `AnalyticsConfig` | `hasbrouck_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4358`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4358) |
| `field` | `AnalyticsConfig` | `almgren_chriss_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4360`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4360) |
| `field` | `AnalyticsConfig` | `acd_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4362`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4362) |
| `field` | `AnalyticsConfig` | `vol_signature_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4364) |
| `field` | `AnalyticsConfig` | `agent_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4366`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4366) |
| `field` | `AnalyticsConfig` | `agent_min_samples` | `: u32` | [`crates/of_core/src/lib.rs:4368`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4368) |
| `field` | `AnalyticsConfig` | `agent_small_trade_threshold` | `: f64` | [`crates/of_core/src/lib.rs:4370`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4370) |
| `field` | `AnalyticsConfig` | `institutional_trade_threshold` | `: i64` | [`crates/of_core/src/lib.rs:4372`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4372) |
| `field` | `AnalyticsConfig` | `institutional_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4374`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4374) |
| `field` | `AnalyticsConfig` | `resiliency_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4376`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4376) |
| `field` | `AnalyticsConfig` | `spread_decomp_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4378`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4378) |
| `field` | `AnalyticsConfig` | `regime_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4380`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4380) |
| `field` | `AnalyticsConfig` | `cancel_arrival_window_ns` | `: u64` | [`crates/of_core/src/lib.rs:4382`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4382) |
| `field` | `AnalyticsConfig` | `event_tracker_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4384) |
| `field` | `AnalyticsConfig` | `spread_tracker_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4386`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4386) |
| `field` | `AnalyticsConfig` | `default_max_len` | `: u32` | [`crates/of_core/src/lib.rs:4388`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_core/src/lib.rs#L4388) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
