from orderflow import Engine, EngineConfig, StreamKind, Symbol


def main() -> None:
    cfg = EngineConfig(instance_id="py-example")
    with Engine(cfg) as engine:
        symbol = Symbol("CME", "ESM6", depth_levels=10)
        engine.subscribe(
            symbol,
            StreamKind.ANALYTICS,
            callback=lambda ev: print("analytics event:", ev),
        )
        engine.subscribe(
            symbol,
            StreamKind.HEALTH,
            callback=lambda ev: print("health event:", ev),
        )
        engine.poll_once()
        print("analytics:", engine.analytics_snapshot(symbol))
        print("signal:", engine.signal_snapshot(symbol))
        print("metrics:", engine.metrics())


if __name__ == "__main__":
    main()
