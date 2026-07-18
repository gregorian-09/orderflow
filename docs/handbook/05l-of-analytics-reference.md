# `of_analytics` Reference

`of_analytics` is the additive advanced analytics crate for Orderflow. It
keeps heavier microstructure modules out of `of_core` so users who only need
canonical market-data types and the basic accumulator do not pay compile-time
or dependency cost for every advanced model.

The first public slice is dependency-light and live-path friendly:

- market-quality/TCA analytics,
- liquidity/depth analytics,
- feature profiles for future impact, toxicity, volatility, regime, pattern,
  derivatives, institutional, and ML-feature modules.

The crate consumes normalized `of_core` data and returns typed snapshots. It
does not own runtime state, persistence, sockets, bindings, OMS state, or order
submission.

## Feature Profiles

Default features:

- `market-quality`
- `liquidity`

Reserved additive profiles:

- `impact`
- `toxicity`
- `volatility`
- `regime`
- `patterns`
- `derivatives`
- `institutional`
- `ml-features`
- `all`

## Public Types

Market quality:

- `AnalyticsError`
- `QuoteContext`
- `TradeContext`
- `MarketQualitySnapshot`
- `MarketQualityTracker`

Liquidity/depth:

- `LiquidityDepthSnapshot`
- `LiquidityDepthAnalyzer`

## Market Quality

`MarketQualityTracker` keeps the latest quote and evaluates a trade against
that quote. It reports:

- quoted spread,
- quoted spread in basis points,
- effective spread in basis points,
- realized spread in basis points when a future midpoint is supplied,
- price improvement in basis points,
- stale-quote flag.

Quote freshness is explicit through `max_quote_age_ns`. If trade/quote age
exceeds that window, the snapshot marks the quote stale instead of hiding the
data-quality problem.

```rust
use of_analytics::{MarketQualityTracker, QuoteContext, TradeContext};
use of_core::Side;

let mut tracker = MarketQualityTracker::new(1_000_000);
tracker.on_quote(QuoteContext::new(499_000, 501_000, 100, 120, 1_000)?);
let snapshot = tracker.evaluate_trade(
    TradeContext::new(500_000, 10, Side::Ask, 1_500)?,
    Some(500_010),
)?;

assert_eq!(snapshot.quoted_spread(), 2_000);
assert!(snapshot.price_improvement_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Liquidity And Depth

`LiquidityDepthAnalyzer` evaluates borrowed `of_core::BookLevel` slices. It
does not copy the book and does not allocate in the analysis path.

The snapshot reports:

- levels used,
- top bid and ask quantity,
- cumulative bid depth,
- cumulative ask depth,
- bid-minus-ask proportional imbalance,
- simple depth slope proxy,
- quantity sweepable by a buy order up to a target quantity,
- quantity sweepable by a sell order up to a target quantity.

```rust
use of_analytics::LiquidityDepthAnalyzer;
use of_core::BookLevel;

let bids = [
    BookLevel { level: 0, price: 499_975, size: 100 },
    BookLevel { level: 1, price: 499_950, size: 80 },
];
let asks = [
    BookLevel { level: 0, price: 500_025, size: 120 },
    BookLevel { level: 1, price: 500_050, size: 90 },
];
let snapshot = LiquidityDepthAnalyzer::new(2).analyze(&bids, &asks, 150)?;

assert_eq!(snapshot.bid_depth(), 180);
assert_eq!(snapshot.ask_depth(), 210);
assert_eq!(snapshot.sweepable_buy_qty(), 150);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Boundary

`of_analytics` is not a migration break:

- existing `of_core` APIs remain valid;
- existing runtime/binding analytics APIs remain valid;
- advanced modules can be wired into runtime and bindings additively later;
- hosts can choose feature profiles based on deployment needs.

This crate is the implementation home for heavier analytics, not a forced
replacement for existing users.
