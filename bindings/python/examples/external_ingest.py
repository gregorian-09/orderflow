from orderflow import (
    BookAction,
    Engine,
    EngineConfig,
    ExternalFeedPolicy,
    Side,
    StreamKind,
    Symbol,
)


def main() -> None:
    cfg = EngineConfig(instance_id="py-external-ingest")
    with Engine(cfg) as engine:
        symbol = Symbol("CME", "ESM6", depth_levels=10)
        engine.subscribe(
            symbol,
            StreamKind.ANALYTICS,
            callback=lambda ev: print("analytics event:", ev),
        )
        engine.configure_external_feed(
            ExternalFeedPolicy(stale_after_ms=2_000, enforce_sequence=True)
        )

        engine.ingest_book(
            symbol,
            side=Side.BID,
            level=0,
            price=504900,
            size=20,
            action=BookAction.UPSERT,
            sequence=1,
            ts_exchange_ns=1_000,
            ts_recv_ns=1_100,
        )
        engine.ingest_trade(
            symbol,
            price=505000,
            size=7,
            aggressor_side=Side.ASK,
            sequence=2,
            ts_exchange_ns=1_200,
            ts_recv_ns=1_300,
        )

        print("analytics:", engine.analytics_snapshot(symbol))
        print("signal:", engine.signal_snapshot(symbol))


if __name__ == "__main__":
    main()
